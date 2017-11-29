#[macro_use]
extern crate clap;
extern crate git2;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustache;
extern crate yaml_rust;

use clap::App;
use std::collections::HashMap;
use git2::Repository;
use rustache::{Render, HashBuilder};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::str;
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
    env_logger::init().unwrap();

    let yaml = load_yaml!("cli.yml");
    let args = App::from_yaml(yaml).get_matches();

    if args.is_present("update") {
        download_sources();
    }

    // builder ->
    // clean previous execution
    build_themes();
}

fn download_sources() {
    match fs::metadata("sources.yaml") {
        Ok(_) => {}
        Err(_) => panic!("sources.yaml not found"),
    };
    let sources = &read_yaml_file("sources.yaml")[0];

    for (source, repo) in sources.as_hash().unwrap().iter() {
        git_clone(
            repo.as_str().unwrap(),
            format!("sources/{}", source.as_str().unwrap()).as_str(),
        );
    }

    match fs::metadata("sources/schemes/list.yaml") {
        Ok(_) => {}
        Err(_) => panic!("sources/schemes/list.yaml not found"),
    };
    let sources_list = &read_yaml_file("sources/schemes/list.yaml")[0];
    for (source, repo) in sources_list.as_hash().unwrap().iter() {
        git_clone(
            repo.as_str().unwrap(),
            format!("schemes/{}", source.as_str().unwrap()).as_str(),
        );
    }

    match fs::metadata("sources/templates/list.yaml") {
        Ok(_) => {}
        Err(_) => panic!("sources/templates/list.yaml not found"),
    };
    let templates_list = &read_yaml_file("sources/templates/list.yaml")[0];
    for (source, repo) in templates_list.as_hash().unwrap().iter() {
        git_clone(
            repo.as_str().unwrap(),
            format!("templates/{}", source.as_str().unwrap()).as_str(),
        );
    }
}

fn build_themes() {
    let templates = get_templates();

    let schemes = get_schemes();

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

                data = data.insert(base.to_string() + "-hex-r", color[0..2].to_string());
                let red = i32::from_str_radix(color[0..2].as_ref(), 16).unwrap();
                data = data.insert(base.to_string() + "-rgb-r", red);
                data = data.insert(base.to_string() + "-dec-r", red / 255);

                data = data.insert(base.to_string() + "-hex-g", color[2..4].to_string());
                let green = i32::from_str_radix(color[2..4].as_ref(), 16).unwrap();
                data = data.insert(base.to_string() + "-rgb-g", green);
                data = data.insert(base.to_string() + "-dec-g", green / 255);

                data = data.insert(base.to_string() + "-hex-b", color[4..6].to_string());
                let blue = i32::from_str_radix(color[4..6].as_ref(), 16).unwrap();
                data = data.insert(base.to_string() + "-rgb-b", blue);
                data = data.insert(base.to_string() + "-dec-b", blue / 255);
            }

            let _ = fs::create_dir(format!("{}", t.output));
            let filename = format!(
                "{}/base16-{}{}",
                t.output,
                s.slug.to_lowercase().replace(" ", "_"),
                t.extension
            );
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
        let template_config = &read_yaml_file(
            format!("{}/templates/config.yaml", template_dir_path).as_str(),
        )
            [0];
        for (config, data) in template_config.as_hash().unwrap().iter() {
            let template_path = format!(
                "{}/templates/{}.mustache",
                template_dir_path.to_string(),
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
                extension: data.as_hash()
                    .unwrap()
                    .get(&Yaml::from_str("extension"))
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
                output: template_dir_path.to_string() + "/" +
                    data.as_hash()
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

                        let slug = &read_yaml_file(scheme_file.to_str().unwrap())[0];
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

fn read_yaml_file(file: &str) -> Vec<yaml_rust::Yaml> {
    debug!("Reading YAML file {}", file);
    let mut src_file = File::open(file).unwrap();
    let mut srcs = String::new();
    src_file.read_to_string(&mut srcs).unwrap();

    let sources = YamlLoader::load_from_str(&mut srcs).unwrap();
    sources
}

fn git_clone(url: &str, path: &str) {
    match fs::metadata(path) {
        Ok(_) => {
            info!("Updating repo at {}", path);
            match Repository::open(path) {
                Ok(repo) => {
                    let _ = repo.find_remote("origin").unwrap().fetch(
                        &["master"],
                        None,
                        None,
                    );
                    let oid = repo.refname_to_id("refs/remotes/origin/master").unwrap();
                    let object = repo.find_object(oid, None).unwrap();
                    repo.reset(&object, git2::ResetType::Hard, None).unwrap()
                }
                Err(e) => panic!("Failed to update: {}", e),
            };
        }
        Err(_) => {
            info!("Cloning repo {}", url);
            match Repository::clone(url, path) {
                Ok(_) => {}
                Err(e) => panic!("Failed to clone: {}", e),
            };
        }
    };
}
