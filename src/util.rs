use std::fs;

use crate::error::Error;

pub fn copy_dir(src: &str, dst: &str) -> Result<(), Error> {
    fs::create_dir_all(dst)?;

    for file in fs::read_dir(src)? {
        let src_file = file?;
        let path = src_file.path();

        if path.is_dir() {
            let sub_dst = format!("{}/{}", dst, path.file_name().unwrap().to_str().unwrap());
            copy_dir(path.to_str().unwrap(), &sub_dst)?;
        } else {
            let dst_file = format!("{}/{}", dst, path.file_name().unwrap().to_str().unwrap());
            fs::copy(&path, &dst_file)?;
        }
    }

    Ok(())
}
