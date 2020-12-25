#[macro_use]
extern crate clap;
extern crate git2;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustache;
extern crate yaml_rust;

use clap::App;
use git2::Repository;
use rustache::{HashBuilder, Render};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read};
use std::path::MAIN_SEPARATOR;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug)]
struct Template {
    data: String,
    extension: String,
    output: String,
}

#[derive(Debug)]
struct Scheme {
    slug: String,
    name: String,
    author: String,
    colors: HashMap<String, String>,
}

fn main() {
    env_logger::init();

    let yaml = load_yaml!("cli.yml");
    let args = App::from_yaml(yaml).get_matches();

    if args.is_present("update") {
        download_sources();
    }

    // TODO: clean previous execution
    build_themes();
}

fn download_sources() {
    match fs::metadata("sources.yaml") {
        Ok(_) => {}
        Err(_) => panic!("sources.yaml not found"),
    };
    let sources = &read_yaml_file("sources.yaml".to_string())[0];

    for (source, repo) in sources.as_hash().unwrap().iter() {
        git_clone(
            repo.as_str().unwrap().to_string(),
            format!("sources{}{}", MAIN_SEPARATOR, source.as_str().unwrap()),
        );
    }

    match fs::metadata(format!(
        "sources{}schemes{}list.yaml",
        MAIN_SEPARATOR, MAIN_SEPARATOR
    )) {
        Ok(_) => {}
        Err(_) => panic!("sources/schemes/list.yaml not found"),
    };
    let sources_list = &read_yaml_file(format!(
        "sources{}schemes{}list.yaml",
        MAIN_SEPARATOR, MAIN_SEPARATOR
    ))[0];
    for (source, repo) in sources_list.as_hash().unwrap().iter() {
        git_clone(
            repo.as_str().unwrap().to_string(),
            format!("schemes{}{}", MAIN_SEPARATOR, source.as_str().unwrap()),
        );
    }

    match fs::metadata(format!(
        "sources{}templates{}list.yaml",
        MAIN_SEPARATOR, MAIN_SEPARATOR
    )) {
        Ok(_) => {}
        Err(_) => panic!("sources/templates/list.yaml not found"),
    };
    let templates_list = &read_yaml_file(format!(
        "sources{}templates{}list.yaml",
        MAIN_SEPARATOR, MAIN_SEPARATOR
    ))[0];
    for (source, repo) in templates_list.as_hash().unwrap().iter() {
        git_clone(
            repo.as_str().unwrap().to_string(),
            format!("templates{}{}", MAIN_SEPARATOR, source.as_str().unwrap()),
        );
    }
}

fn build_themes() {
    let templates = get_templates();
    let schemes = get_schemes();

    let mut filenames = HashSet::new();

    for s in &schemes {
        for t in &templates {
            info!(
                "Building {}/base16-{}{}",
                t.output,
                s.slug.to_string(),
                t.extension
            );
            let mut data = HashBuilder::new();
            data = data.insert("scheme-slug", s.slug.as_ref());
            data = data.insert("scheme-name", s.name.as_ref());
            data = data.insert("scheme-author", s.author.as_ref());

            for (base, color) in &s.colors {
                data = data.insert(base.to_string() + "-hex", color.as_ref());

                let hex_red = color[0..2].to_string();
                data = data.insert(base.to_string() + "-hex-r", hex_red.as_ref());
                let red = i32::from_str_radix(color[0..2].as_ref(), 16).unwrap();
                data = data.insert(base.to_string() + "-rgb-r", red);
                data = data.insert(base.to_string() + "-dec-r", red / 255);

                let hex_green = color[2..4].to_string();
                data = data.insert(base.to_string() + "-hex-g", hex_green.as_ref());
                let green = i32::from_str_radix(color[2..4].as_ref(), 16).unwrap();
                data = data.insert(base.to_string() + "-rgb-g", green);
                data = data.insert(base.to_string() + "-dec-g", green / 255);

                let hex_blue = color[4..6].to_string();
                data = data.insert(base.to_string() + "-hex-b", hex_blue.as_ref());
                let blue = i32::from_str_radix(color[4..6].as_ref(), 16).unwrap();
                data = data.insert(base.to_string() + "-rgb-b", blue);
                data = data.insert(base.to_string() + "-dec-b", blue / 255);

                data = data.insert(
                    base.to_string() + "-hex-bgr",
                    format!("{}{}{}", hex_blue, hex_green, hex_red),
                );
            }

            let _ = fs::create_dir(t.output.to_string());
            let filename = format!(
                "{}{}base16-{}{}",
                t.output,
                MAIN_SEPARATOR,
                s.slug.to_lowercase().replace(" ", "_"),
                t.extension
            );

            if filenames.contains(&filename) {
                println!("\nWarning: {} was overwritten.", filename);
            } else {
                filenames.insert(filename.to_string());
            }

            let f = File::create(filename).unwrap();
            let mut out = BufWriter::new(f);
            data.render(&t.data, &mut out).unwrap();
            println!("Built base16-{}{}", s.slug, t.extension);
        }
    }
}

fn get_templates() -> Vec<Template> {
    let mut templates = vec![];

    for template_dir in fs::read_dir("templates").unwrap() {
        let template_dir = template_dir.unwrap().path();
        let template_dir_path = template_dir.to_str().unwrap();
        let template_config = &read_yaml_file(format!(
            "{}{}templates{}config.yaml",
            template_dir_path, MAIN_SEPARATOR, MAIN_SEPARATOR
        ))[0];
        for (config, data) in template_config.as_hash().unwrap().iter() {
            let template_path = format!(
                "{}{}templates{}{}.mustache",
                template_dir_path.to_string(),
                MAIN_SEPARATOR,
                MAIN_SEPARATOR,
                config.as_str().unwrap()
            );
            info!("Reading template {}", template_path);

            let template_data = {
                let mut d = String::new();
                let f = File::open(template_path).unwrap();
                let mut input = BufReader::new(f);
                input.read_to_string(&mut d).unwrap();
                d
            };

            let template = Template {
                data: template_data,
                extension: data
                    .as_hash()
                    .unwrap()
                    .get(&Yaml::from_str("extension"))
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                output: template_dir_path.to_string()
                    + MAIN_SEPARATOR.to_string().as_str()
                    + data
                        .as_hash()
                        .unwrap()
                        .get(&Yaml::from_str("output"))
                        .unwrap()
                        .as_str()
                        .unwrap(),
            };

            templates.push(template);
        }
    }
    templates
}

fn get_schemes() -> Vec<Scheme> {
    let mut schemes = vec![];

    let schemes_dir = fs::read_dir("schemes").unwrap();
    for scheme in schemes_dir {
        let scheme_files = fs::read_dir(scheme.unwrap().path()).unwrap();
        for sf in scheme_files {
            let scheme_file = sf.unwrap().path();
            match scheme_file.extension() {
                None => {}
                Some(ext) => {
                    if ext == "yaml" {
                        info!("Reading scheme {}", scheme_file.display());
                        let mut scheme_name = String::new();
                        let mut scheme_author = String::new();
                        let mut scheme_colors: HashMap<String, String> = HashMap::new();

                        let slug = &read_yaml_file(scheme_file.to_string_lossy().into_owned())[0];
                        for (attr, value) in slug.as_hash().unwrap().iter() {
                            let v = value.as_str().unwrap().to_string();
                            match attr.as_str().unwrap() {
                                "scheme" => {
                                    scheme_name = v;
                                }
                                "author" => {
                                    scheme_author = v;
                                }
                                _ => {
                                    scheme_colors.insert(attr.as_str().unwrap().to_string(), v);
                                }
                            };
                        }

                        let sc = Scheme {
                            name: scheme_name,
                            author: scheme_author,
                            slug: scheme_file
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string(),
                            colors: scheme_colors,
                        };

                        schemes.push(sc);
                    }
                }
            };
        }
    }

    schemes
}

fn read_yaml_file(file: String) -> Vec<yaml_rust::Yaml> {
    debug!("Reading YAML file {}", file);
    let mut src_file = File::open(file).unwrap();
    let mut srcs = String::new();
    src_file.read_to_string(&mut srcs).unwrap();

    YamlLoader::load_from_str(&srcs).unwrap()
}

fn git_clone(url: String, path: String) {
    println!("-- {}", path);
    match fs::metadata(path.clone()) {
        Ok(_) => {
            info!("Updating repo at {}", path);
            match Repository::open(path) {
                Ok(repo) => {
                    let _ = repo
                        .find_remote("origin")
                        .unwrap()
                        .fetch(&["master"], None, None);
                    let oid = repo.refname_to_id("refs/remotes/origin/master").unwrap();
                    let object = repo.find_object(oid, None).unwrap();
                    repo.reset(&object, git2::ResetType::Hard, None).unwrap()
                }
                Err(e) => panic!("Failed to update: {}", e),
            };
        }
        Err(_) => {
            info!("Cloning repo {}", url);
            match Repository::clone(url.as_str(), path) {
                Ok(_) => {}
                Err(e) => panic!("Failed to clone: {}", e),
            };
        }
    };
}
