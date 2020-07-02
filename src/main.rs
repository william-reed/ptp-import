extern crate chrono;
extern crate libptp;
extern crate rusb;

use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Add;
use std::path::Path;
use std::time::Duration;

use chrono::{Datelike, NaiveDate};
use libptp::Camera;
use rusb::UsbContext;

const FOLDER_OBJECT_FORMAT: u16 = 0x3001;

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

    // TODO can probably figure out which are PTP devices a more appropriate way
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

                let file_size_mebibytes = (info.ObjectCompressedSize as f32) / 1024.0 / 1024.0;
                if file_size_mebibytes > 50.0 {
                    println!("too big: {}", file_size_mebibytes);
                    // continue
                }

                let date =
                    match NaiveDate::parse_from_str(info.CaptureDate.as_str(), "%Y%m%dT%H%M%S") {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("Could not parse date {}: {}", info.CaptureDate, e);
                            continue;
                        }
                    };

                println!("{} ({:.2} MiB) from {}", info.Filename, file_size_mebibytes, date.format("%d/%m/%Y").to_string());

                match save_file(info.Filename, date, &mut cam, handle) {
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
/// - `handle` - the object handle
///
/// # Return
/// `()` or a `libptp::Error`. Using the `ptp` version since it can convert io errors to itself. Should
/// probably have my own error type that wraps all of them?
fn save_file(filename: String, date: NaiveDate, cam: &mut Camera<rusb::Context>, handle: u32) -> Result<(), libptp::Error> {
    let path = format!("{}/{}/{}", date.year(), date.month(), date.day());
    let file_and_path = path.clone().add(format!("/{}", filename).as_str());

    // create path if it doesn't exist
    if !Path::new(path.as_str()).exists() {
        fs::create_dir_all(&path)?;
    }

    // TODO prevent overwrite if photo exists, or use duplicate naming scheme

    let mut file = File::create(file_and_path)?;
    let data = cam.get_object(handle, None)?;
    file.write_all(data.as_slice())?;
    Ok(())
}