extern crate git2;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustache;
extern crate yaml_rust;

use git2::Repository;
use rustache::{Render, HashBuilder};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::str;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug)]
struct Template {
    dir: String,
    slug: String,
    extension: String,
    output: String,
    data: String,
}

fn main() {
    env_logger::init().unwrap();

    // builder update ->
    download_sources();

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
        git_clone(repo.as_str().unwrap(),
                  format!("sources/{}", source.as_str().unwrap()).as_str());
    }

    match fs::metadata("sources/schemes/list.yaml") {
        Ok(_) => {}
        Err(_) => panic!("sources/schemes/list.yaml not found"),
    };
    let sources_list = &read_yaml_file("sources/schemes/list.yaml")[0];
    for (source, repo) in sources_list.as_hash().unwrap().iter() {
        git_clone(repo.as_str().unwrap(),
                  format!("schemes/{}", source.as_str().unwrap()).as_str());
    }

    match fs::metadata("sources/templates/list.yaml") {
        Ok(_) => {}
        Err(_) => panic!("sources/templates/list.yaml not found"),
    };
    let templates_list = &read_yaml_file("sources/templates/list.yaml")[0];
    for (source, repo) in templates_list.as_hash().unwrap().iter() {
        git_clone(repo.as_str().unwrap(),
                  format!("templates/{}", source.as_str().unwrap()).as_str());
    }
}

fn build_themes() {
    let mut vec = vec![];
    {
        for template_dir in fs::read_dir("templates").unwrap() {
            let template_dir = template_dir.unwrap();
            let template_path = template_dir.path();
            let template_path = template_path.to_str();
            let template_config = &read_yaml_file(format!("{}/templates/config.yaml",
                                                          template_path.unwrap())
                .as_str())[0];
            for (slug, data) in template_config.as_hash().unwrap().iter() {
                let template_data = {
                    let mut d = String::new();
                    info!("Reading template {}/templates/{}.mustache",
                          template_path.unwrap().to_string(),
                          slug.as_str().unwrap());
                    let f = File::open(format!("{}/templates/{}.mustache",
                                               template_path.unwrap().to_string(),
                                               slug.as_str().unwrap()))
                        .unwrap();
                    let mut input = BufReader::new(f);
                    input.read_to_string(&mut d).unwrap();
                    d
                };

                let template = Template {
                    dir: template_path.unwrap().to_string(),
                    slug: slug.as_str().unwrap().to_string(),
                    extension: data.as_hash()
                        .unwrap()
                        .get(&Yaml::from_str("extension"))
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    output: data.as_hash()
                        .unwrap()
                        .get(&Yaml::from_str("output"))
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    data: template_data,
                };
                vec.push(template);
            }
        }

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
                            let mut data = HashBuilder::new();
                            let s = &read_yaml_file(scheme_file.to_str().unwrap())[0];
                            for (attr, value) in s.as_hash().unwrap().iter() {
                                match attr.as_str().unwrap() {
                                    "scheme" => {
                                        data = data.insert("scheme-name", value.as_str().unwrap());
                                    }
                                    "author" => {
                                        data =
                                            data.insert("scheme-author", value.as_str().unwrap());
                                    }
                                    _ => {
                                        let key = attr.as_str().unwrap();
                                        let v = value.as_str().unwrap();
                                        data = data.insert(key.to_string() + "-hex", v);
                                        data = data.insert(key.to_string() + "-hex-r",
                                                           v[0..2].to_string());
                                        data = data.insert(key.to_string() + "-rgb-r",
                                                           i32::from_str_radix(v[0..2].as_ref(),
                                                                               16)
                                                               .unwrap());
                                        data = data.insert(key.to_string() + "-hex-g",
                                                           v[2..4].to_string());
                                        data = data.insert(key.to_string() + "-rgb-g",
                                                           i32::from_str_radix(v[2..4].as_ref(),
                                                                               16)
                                                               .unwrap());
                                        data = data.insert(key.to_string() + "-hex-b",
                                                           v[4..6].to_string());
                                        data = data.insert(key.to_string() + "-rgb-b",
                                                           i32::from_str_radix(v[4..6].as_ref(),
                                                                               16)
                                                               .unwrap());
                                    }
                                };
                            }

                            for t in &vec {
                                info!("Building {}/{}/base16-{}{}",
                                      t.dir,
                                      t.output,
                                      scheme_file.file_stem().unwrap().to_str().unwrap(),
                                      t.extension);
                                data = data.insert("scheme-slug", t.slug.as_ref());
                                fs::create_dir(format!("{}/{}", t.dir, t.output));
                                let f = File::create(format!("{}/{}/base16-{}{}",
                                                             t.dir,
                                                             t.output,
                                                             scheme_file.file_stem()
                                                                 .unwrap()
                                                                 .to_str()
                                                                 .unwrap(),
                                                             t.extension))
                                    .unwrap();
                                let mut out = BufWriter::new(f);
                                data.render(&t.data, &mut out).unwrap();
                            }
                        }
                    }
                };
            }
        }
    }
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
            // TODO: implement "git pull"
            info!("Updating repo at {}", path);
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
