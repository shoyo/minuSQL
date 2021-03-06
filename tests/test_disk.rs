/*
 * Copyright (c) 2020 - 2021.  Shoyo Inokuchi.
 * Please refer to github.com/shoyo/jindb for more information about this project and its license.
 */

use jin::constants::{CATALOG_ROOT_ID, PAGE_SIZE};
use jin::disk::{open_write_file, DiskManager};
use std::convert::TryInto;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Barrier};
use std::thread;

struct TestContext {
    disk_manager: DiskManager,
    filename: String,
}

impl Drop for TestContext {
    fn drop(&mut self) {
        fs::remove_file(&self.filename).unwrap();
    }
}

fn setup(test_id: usize) -> TestContext {
    let filename = format!("DM_TEST_{}", test_id);
    TestContext {
        disk_manager: DiskManager::new(&filename),
        filename,
    }
}

#[test]
fn test_disk_allocation() {
    let mut ctx = setup(0);
    let manager = &mut ctx.disk_manager;

    assert_eq!(manager.is_allocated(CATALOG_ROOT_ID), true);
    assert_eq!(manager.is_allocated(CATALOG_ROOT_ID + 1), false);

    let page_id = manager.allocate_page();
    assert_eq!(page_id, CATALOG_ROOT_ID + 1);
    assert_eq!(manager.is_allocated(CATALOG_ROOT_ID + 1), true);
}

#[test]
fn test_disk_write() {
    let ctx = setup(1);

    // Write expected data to disk with disk manager.
    let expected = [123; PAGE_SIZE as usize];
    let page_id = ctx.disk_manager.allocate_page();
    ctx.disk_manager.write_page(page_id, &expected);

    // Manually read page data from disk.
    let mut actual = [0; PAGE_SIZE as usize];
    let mut file = File::open(&ctx.filename).unwrap();
    file.seek(SeekFrom::Start((page_id * PAGE_SIZE) as u64))
        .unwrap();
    file.read_exact(&mut actual).unwrap();
    file.flush().unwrap();

    // Assert that actual data matches expected data.
    for i in 0..PAGE_SIZE as usize {
        assert_eq!(actual[i], expected[i]);
    }
}

#[test]
fn test_disk_read() {
    let ctx = setup(2);

    // Manually write page data to disk.
    let mut file = open_write_file(&ctx.filename);
    let page_id = ctx.disk_manager.allocate_page();
    file.seek(SeekFrom::Start((page_id * PAGE_SIZE) as u64))
        .unwrap();
    for i in 0..=255 {
        let byte = file.write(&[i]).unwrap();
        assert_eq!(byte, 1);
    }

    // Read page data from disk with disk manager.
    let mut data = [0; PAGE_SIZE as usize];
    ctx.disk_manager.read_page(page_id, &mut data);

    // Assert that actual data matches expected data.
    for i in 0..=255 {
        assert_eq!(data[i], i as u8);
    }
}

#[test]
#[should_panic]
fn test_unallocated_read() {
    let ctx = setup(3);
    ctx.disk_manager.read_page(2, &mut [0; PAGE_SIZE as usize]);
}

#[test]
#[should_panic]
fn test_unallocated_write() {
    let ctx = setup(4);
    ctx.disk_manager.write_page(2, &[0; PAGE_SIZE as usize]);
}

#[test]
/// Assert that multiple threads can read the same page from disk simultaneously.
fn test_concurrent_read_access() {
    let ctx = Arc::new(setup(5));
    let num_threads = 10;

    // Write data to a page on disk.
    let page_id = ctx.disk_manager.allocate_page();
    let expected = [213; PAGE_SIZE as usize];
    ctx.disk_manager.write_page(page_id, &expected);

    // Spin up multiple threads, and make each thread independently read the same page into
    // memory. Assert that each thread obtains the correct data.
    for _ in 0..num_threads {
        let ctx_c = ctx.clone();
        thread::spawn(move || {
            let mut actual = [0; PAGE_SIZE as usize];
            ctx_c.disk_manager.read_page(page_id, &mut actual);

            for i in 0..PAGE_SIZE as usize {
                assert_eq!(actual[i], expected[i]);
            }
        });
    }
}

#[test]
/// Assert that multiple threads can allocate and write to different pages on disk
/// simultaneously.
fn test_concurrent_write_access() {
    let ctx = Arc::new(setup(6));
    let num_threads = 100;

    // Spin up multiple threads, and make each thread allocate a new page on disk.
    // Have each thread write some unique data to their corresponding page.
    let mut handles = Vec::with_capacity(num_threads);

    for _ in 0..num_threads {
        let ctx_c = ctx.clone();
        handles.push(thread::spawn(move || {
            let page_id = ctx_c.disk_manager.allocate_page();

            // Write the page's ID to each byte of the newly allocated page.
            ctx_c
                .disk_manager
                .write_page(page_id, &[page_id.try_into().unwrap(); PAGE_SIZE as usize]);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Assert that allocations were successful.
    assert!(ctx.disk_manager.is_allocated(num_threads as u32));

    // Spin up a new set of threads, and make all threads access a different disk page
    // simultaneously. Assert that each page contains the correct data.
    let mut handles = Vec::with_capacity(num_threads);
    let barrier = Arc::new(Barrier::new(num_threads));

    for i in 1..(num_threads + 1) as u32 {
        let ctx_c = ctx.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            let mut data = [0; PAGE_SIZE as usize];

            bar.wait(); // Sync all threads

            // Assert that each byte of the page is the page's ID.
            ctx_c.disk_manager.read_page(i, &mut data);

            for j in 0..PAGE_SIZE as usize {
                assert_eq!(data[j], i as u8);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
