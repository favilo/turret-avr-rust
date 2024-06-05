use std::{collections::HashMap, path::PathBuf};

use cc::Build;

const CONFIG_FILE: &str = "arduino.yaml";

#[derive(Debug, serde::Deserialize)]
struct BindgenLists {
    pub allowlist_function: Vec<String>,
    pub allowlist_type: Vec<String>,
    pub blocklist_function: Vec<String>,
    pub blocklist_type: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Config {
    pub arduino_home: String,
    pub external_libraries_home: String,
    pub core_version: String,
    pub variant: Option<String>,
    pub avr_gcc_version: String,
    pub arduino_libraries: Vec<String>,
    pub external_libraries: Vec<String>,
    pub external_library_files: Vec<String>,

    pub excluded_headers: Vec<String>,

    pub definitions: HashMap<String, String>,
    pub flags: Vec<String>,

    pub bindgen_lists: BindgenLists,
}

impl Config {
    fn arduino_package_path(&self) -> PathBuf {
        let expanded = envmnt::expand(&self.arduino_home, None);
        let arduino_home = PathBuf::from(&expanded);
        arduino_home.join("packages").join("arduino")
    }

    fn core_path(&self) -> PathBuf {
        self.arduino_package_path()
            .join("hardware")
            .join("avr")
            .join(&self.core_version)
    }

    fn avr_gcc_home(&self) -> PathBuf {
        self.arduino_package_path()
            .join("tools")
            .join("avr-gcc")
            .join(&self.avr_gcc_version)
    }

    fn avr_gcc(&self) -> PathBuf {
        self.avr_gcc_home().join("bin").join("avr-gcc")
    }

    fn arduino_core_path(&self) -> PathBuf {
        self.core_path().join("cores").join("arduino")
    }

    fn arduino_include_dirs(&self) -> Vec<PathBuf> {
        let variant_path = self
            .core_path()
            .join("variants")
            .join(&self.variant.as_deref().unwrap_or("standard"));
        let avr_gcc_include_path = self.avr_gcc_home().join("avr").join("include");
        vec![self.arduino_core_path(), variant_path, avr_gcc_include_path]
    }

    fn arduino_libraries_path(&self) -> Vec<PathBuf> {
        let library_root = self.arduino_package_path().join("libraries");
        self.arduino_libraries
            .iter()
            .map(|lib| library_root.join(lib).join("src"))
            .collect()
    }

    fn external_libraries_path(&self) -> Vec<PathBuf> {
        let expanded = envmnt::expand(&self.external_libraries_home, None);
        let external_libraries_root = PathBuf::from(&expanded);
        self.external_libraries
            .iter()
            .map(|lib| external_libraries_root.join(lib).join("src"))
            .collect()
    }

    fn include_dirs(&self) -> Vec<PathBuf> {
        let mut include_dirs = self.arduino_include_dirs();
        include_dirs.extend(self.arduino_libraries_path());
        include_dirs.extend(self.external_libraries_path());
        include_dirs
    }

    fn project_files(&self, pattern: &str) -> Vec<PathBuf> {
        let mut result =
            files_in_folder(self.arduino_core_path().to_string_lossy().as_ref(), pattern);
        let mut libraries = self.arduino_libraries_path();
        libraries.extend(self.external_libraries_path());

        let pattern = format!("**/{}", pattern);
        for library in libraries {
            result.extend(files_in_folder(
                library.to_string_lossy().as_ref(),
                &pattern,
            ));
        }

        result
    }

    fn cpp_files(&self) -> Vec<PathBuf> {
        let mut files = self.project_files("*.cpp");
        files.extend(
            self.external_library_files
                .iter()
                .filter(|file| file.ends_with(".cpp"))
                .map(|file| PathBuf::from("c_libraries").join(file)),
        );
        files
    }

    fn c_files(&self) -> Vec<PathBuf> {
        let mut files = self.project_files("*.c");
        files.extend(
            self.external_library_files
                .iter()
                .filter(|file| file.ends_with(".c"))
                .map(|file| PathBuf::from("c_libraries").join(file)),
        );
        files
    }

    fn bindgen_headers(&self) -> Vec<PathBuf> {
        self.external_libraries_path()
            .iter()
            .map(|lib| files_in_folder(lib.to_string_lossy().as_ref(), "*.h*"))
            .flatten()
            .filter(|file: &PathBuf| {
                !self
                    .excluded_headers
                    .iter()
                    .any(|header| file.ends_with(header))
            })
            .collect()
    }
}

fn files_in_folder(folder: &str, pattern: &str) -> Vec<PathBuf> {
    let cpp_pattern = format!("{folder}/{pattern}");
    glob::glob(&cpp_pattern)
        .unwrap()
        .map(Result::unwrap)
        .filter(|file| {
            !file.ends_with("main.cpp") && !file.to_string_lossy().as_ref().contains("example")
        })
        .collect()
}

fn configure_arduino(config: &Config) -> Build {
    let mut builder = Build::new();
    for (key, value) in &config.definitions {
        builder.define(key, value.as_str());
    }

    for flag in &config.flags {
        builder.flag(flag);
    }
    builder
        .compiler(config.avr_gcc())
        .flag("-Os")
        .cpp_set_stdlib(None)
        .flag("-fno-exceptions")
        .flag("-ffunction-sections")
        .flag("-fdata-sections");

    for include_dir in config.include_dirs() {
        builder.include(include_dir);
    }

    builder
}

fn configure_bindgen_for_arduino(config: &Config) -> bindgen::Builder {
    let mut builder = bindgen::Builder::default();
    for (key, value) in &config.definitions {
        builder = builder.clang_arg(format!("-D{}={}", key, value));
    }
    for flag in &config.flags {
        builder = builder.clang_arg(flag);
    }

    builder = builder
        .clang_args(&["-x", "c++", "-std=gnu++11"])
        .size_t_is_usize(false)
        .use_core()
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    for include_dir in config.include_dirs() {
        builder = builder.clang_arg(format!("-I{}", include_dir.to_string_lossy()));
    }
    for header in dbg!(config.bindgen_headers()) {
        builder = builder.header(header.to_string_lossy());
    }

    for item in &config.bindgen_lists.allowlist_function {
        builder = builder.allowlist_function(item);
    }
    for item in &config.bindgen_lists.allowlist_type {
        builder = builder.allowlist_type(item);
    }
    for item in &config.bindgen_lists.blocklist_function {
        builder = builder.blocklist_function(item);
    }
    for item in &config.bindgen_lists.blocklist_type {
        builder = builder.blocklist_type(item);
    }
    builder
}

fn add_source_file(builder: &mut Build, files: &[PathBuf]) {
    for file in files {
        println!("cargo:rerun-if-changed={}", file.to_string_lossy());
        builder.file(file);
    }
}

fn compile_arduino(config: &Config) {
    let mut builder = configure_arduino(config);
    builder
        .cpp(true)
        .flag("-std=gnu++11")
        .flag("-fpermissive")
        .flag("-fno-threadsafe-statics");
    add_source_file(&mut builder, &config.cpp_files());
    builder.compile("libarduino_c++.a");

    let mut builder = configure_arduino(config);
    builder.flag("-std=gnu11");
    add_source_file(&mut builder, &config.c_files());
    builder.compile("libarduino_c.a");

    // println!("cargo:rustc-link-lib=static=arduino_c++");
    // println!("cargo:rustc-link-lib=static=arduino_c");
}

fn generate_bindings(config: &Config) {
    let bindings = configure_bindgen_for_arduino(config)
        .generate()
        .expect("Unable to generate bindings");
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("arduino.rs");
    bindings
        .write_to_file(project_root)
        .expect("Couldn't write bindings!");
}

fn main() {
    println!("cargo:rerun-if-changed={}", CONFIG_FILE);
    let config_string = std::fs::read_to_string(CONFIG_FILE)
        .unwrap_or_else(|e| panic!("Unable to read {} file: {}", CONFIG_FILE, e));
    let config: Config = serde_yaml::from_str(&config_string)
        .unwrap_or_else(|e| panic!("Unable to parse {} file: {}", CONFIG_FILE, e));

    println!("Arduino configuration: {:#?}", config);
    compile_arduino(&config);
    generate_bindings(&config);
}
