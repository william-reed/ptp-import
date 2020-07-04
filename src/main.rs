extern crate chrono;
extern crate libptp;
extern crate rusb;

use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Add;
use std::path::Path;

use chrono::{Datelike, NaiveDate};
use libptp::Camera;
use rusb::UsbContext;

const FOLDER_OBJECT_FORMAT: u16 = 0x3001;
const MAX_PARTIAL_TRANSFER_BYTES: u32 = 15 * 1024 * 1024; // 15 MiB

/// skip_fail!
///
/// return the unwrapped value, or `continue` the loop and print an error.
///
/// # Arguments
/// - `e`: the optional expression to evaluate
/// - `message`: the message to print along with an error if it occurs
macro_rules! skip_fail {
    ($e: expr, $message: expr) => {
        match $e {
            Ok(val) => val,
            Err(e) => {
                println!("Error occurred: {}, {}; skipped.", $message, e);
                continue;
            }
        }
    };
}

// PTP reference
// https://people.ece.cornell.edu/land/courses/ece4760/FinalProjects/f2012/jmv87/site/files/pima15740-2000.pdf

fn main() {
    let context = rusb::Context::new().unwrap();

    let cams = context.devices().unwrap()
        .iter()
        .filter_map(|d| libptp::Camera::new(&d).ok())
        .collect::<Vec<Camera<rusb::Context>>>();

    for mut cam in cams {
        let info = skip_fail!(cam.get_device_info(None), "Couldn't read camera info");
        println!("{} {}", info.Manufacturer, info.Model);

        skip_fail!(cam.open_session(None), "Could not open session for device.");
        let storage_ids = skip_fail!(cam.get_storageids(None), "Could not get storage ids.");

        for storage_id in storage_ids {
            let handles = skip_fail!(cam.get_objecthandles_all(storage_id, None, None), "Could not get object handles.");
            println!("File count: {}", handles.len());

            for handle in handles {
                let info = skip_fail!(cam.get_objectinfo(handle, None), "Could not get object info from camera.");
                // skip folders
                if info.ObjectFormat == FOLDER_OBJECT_FORMAT { continue; }

                let date = skip_fail!(NaiveDate::parse_from_str(&info.CaptureDate, "%Y%m%dT%H%M%S"), format!("Could not parse date {}", info.CaptureDate));

                println!("{} ({:.2} MiB) from {}", info.Filename, (info.ObjectCompressedSize as f32) / 1024.0 / 1024.0, date.format("%d/%m/%Y").to_string());

                match save_file(info.Filename, date, info.ObjectCompressedSize as u32, &mut cam, handle) {
                    Ok(_) => {}
                    Err(e) => eprintln!("{}", e)
                }
            }
        }

        cam.close_session(None).expect("Could not close camera session.");
    }
}

/// save_file
///
/// # Arguments
/// - `filename` - the image file name
/// - `date` - the date the image was created, to store the file in the proper directory
/// - `cam` - the PTP camera reference
/// - `image_size_bytes` - the image size in bytes
/// - `handle` - the object handle
///
/// # Return
/// `()` or a `libptp::Error`. Using the `ptp` version since it can convert io errors to itself. Should
/// probably have my own error type that wraps all of them?
fn save_file(filename: String, date: NaiveDate, image_size_bytes: u32, cam: &mut Camera<rusb::Context>, handle: u32) -> Result<(), libptp::Error> {
    let path = format!("{}/{}/{}", date.year(), date.month(), date.day());
    let mut file_and_path = path.clone().add(&format!("/{}", filename));

    // create path if it doesn't exist
    if !Path::new(&path).exists() {
        fs::create_dir_all(&path)?;
    }

    // if it does exist, check the image sizes to compare
    if Path::new(&file_and_path).exists() {
        let file_len_bytes = fs::metadata(file_and_path)?.len();
        // TODO can probably use some other information to determine if photo is a possible match
        //  like looking at capture date
        if file_len_bytes == image_size_bytes as u64 {
            println!("\tImage already exists; skipping.");
            return Ok(());
        } else {
            println!("\tImage already exists but doesn't seem to match.");

            // find the next available non-duplicate file name
            let mut ii = 0;
            loop {
                ii += 1;
                file_and_path = path.clone().add(&format!("/{}-{}", filename, ii));

                if !Path::new(&file_and_path).exists() { break; }
            }
            println!("\tUsing {} for image.", file_and_path);
        }
    }

    let mut file = File::create(&file_and_path)?;

    let mut total_transferred_bytes = 0;
    let mut data: Vec<u8> = Vec::new();
    while total_transferred_bytes < image_size_bytes {
        let mut partial_data = cam.get_partialobject(handle, total_transferred_bytes, MAX_PARTIAL_TRANSFER_BYTES, None)?;
        data.append(&mut partial_data);
        total_transferred_bytes += MAX_PARTIAL_TRANSFER_BYTES
    }
    file.write_all(data.as_slice())?;
    Ok(())
}