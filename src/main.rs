extern crate git2;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate rustache;
extern crate yaml_rust;

use git2::Repository;
use rustache::HashBuilder;
use std::fs;
use std::fs::File;
use std::io::Read;
use yaml_rust::YamlLoader;

fn main() {
    env_logger::init().unwrap();

    download_sources();

    build_themes();
}

fn download_sources() {
    match fs::metadata("sources.yaml") {
        Ok(_) => {},
        Err(_) => panic!("sources.yaml not found")
    };
    let sources = &read_yaml_file("sources.yaml")[0];

    for (source, repo) in sources.as_hash().unwrap().iter() {
        git_clone(repo.as_str().unwrap(), format!("sources/{}", source.as_str().unwrap()).as_str());
    }

    match fs::metadata("sources/schemes/list.yaml") {
        Ok(_) => {},
        Err(_) => panic!("sources/schemes/list.yaml not found")
    };
    let sources_list = &read_yaml_file("sources/schemes/list.yaml")[0];
    for (source, repo) in sources_list.as_hash().unwrap().iter() {
        git_clone(repo.as_str().unwrap(), format!("schemes/{}", source.as_str().unwrap()).as_str());
    }

    match fs::metadata("sources/templates/list.yaml") {
        Ok(_) => {},
        Err(_) => panic!("sources/templates/list.yaml not found")
    };
    let templates_list = &read_yaml_file("sources/templates/list.yaml")[0];
    for (source, repo) in templates_list.as_hash().unwrap().iter() {
        git_clone(repo.as_str().unwrap(), format!("templates/{}", source.as_str().unwrap()).as_str());
    }
}

fn build_themes() {
    let mut vec = Vec::new();
    let templates = fs::read_dir("templates").unwrap();
    for template in templates {
        let template_files = fs::read_dir(format!("{}/templates", template.unwrap().path().to_str().unwrap())).unwrap();
        for tf in template_files {
            let template_file = tf.unwrap().path();
            match template_file.extension() {
                None => {},
                Some(ext) => {
                    if ext == "mustache" {
                        info!("Reading template {}", template_file.display());
                        vec.push(template_file.clone());
                    }
                }
            };
        }
    }

    let schemes = fs::read_dir("schemes").unwrap();

    for scheme in schemes {
        let scheme_files = fs::read_dir(scheme.unwrap().path()).unwrap();
        for sf in scheme_files {
            let scheme_file = sf.unwrap().path();
            match scheme_file.extension() {
                None => {},
                Some(ext) => {
                    if ext == "yaml" {
                        info!("Reading scheme {}", scheme_file.display());
                        //let data = HashBuilder::new();
                        //let s = &read_yaml_file(scheme_file.to_str().unwrap())[0];
                        //for (attr, value) in s.as_hash().unwrap().iter() {
                        //    data.insert_string(attr.as_str().unwrap(), value.as_str().unwrap());
                        //}

                        //for t in vec {
                        //    rustache::render_file(t.to_str().unwrap(), data);
                        //}
                    }
                }
            };
        }
    }
}

fn read_yaml_file(file: &str) -> Vec<yaml_rust::Yaml> {
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
        },
        Err(_) => {
            info!("Cloning repo {}", url);
            match Repository::clone(url, path) {
               Ok(_) => {},
               Err(e) => panic!("Failed to clone: {}", e),
            };
        }
    };
}
