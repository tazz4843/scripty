use std::env;

pub fn set_dir() {
    match env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => {
                if let Err(err) = env::set_current_dir(parent) {
                    println!("Couldn't change the current directory: {}", err);
                }
            }
            None => println!("Couldn't get the directory of the exe"),
        },
        Err(err) => println!("Couldn't get the location of the exe: {}", err),
    }
    match env::current_dir() {
        Ok(dir) => println!(
            "All the files and all will be put in or read from: {}",
            dir.display()
        ),
        Err(err) => println!("Couldn't even get the current directory: {}", err),
    }
}
