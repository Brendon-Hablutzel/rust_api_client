use std::{fs, io::Write};

#[macro_export]
macro_rules! err_to_string {
    ($fallible:expr) => {
        $fallible.map_err(|err| err.to_string())
    };
}

pub fn write_to_file(file_path: &str, data: &str) -> Result<(), String> {
    let mut file = err_to_string!(fs::File::open(file_path))?;
    err_to_string!(file.write_all(data.as_bytes()))?;
    Ok(())
}
