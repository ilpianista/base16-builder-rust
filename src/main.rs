#![feature(drain_filter)]
#[macro_use]
extern crate clap;
extern crate git2;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rayon;
extern crate rustache;
extern crate yaml_rust;

use clap::App;
use git2::Repository;
use rayon::prelude::*;
use rustache::{HashBuilder, Render};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read};
use std::path::MAIN_SEPARATOR;
use std::process::exit;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug)]
struct Source<'a> {
    source: &'a str,
    repo: &'a str,
    kind: &'a str,
}

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

    if args.is_present("list") {
        let schemes = get_schemes();
        for scheme in schemes {
            println!("{}", scheme.name);
        }
        exit(0);
    }

    // TODO: clean previous execution
    let theme = args.value_of("theme");
    build_themes(theme);
}

/// Builds a vec of `Source` structs from sources.yml
/// and downloads all of the repos in parallel
fn download_sources() {
    let mut buf1 = Yaml::from_str("-123");
    let mut buf2 = Yaml::from_str("-123");
    let mut buf3 = Yaml::from_str("-123");

    let sources = src_builder("sources.yaml", &mut buf1, "sources");
    src_getter(&sources);

    let sources_list_path: &str =
        &format!("sources{}schemes{}list.yaml", MAIN_SEPARATOR, MAIN_SEPARATOR);
    let templates_list_path: &str = &format!(
        "sources{}templates{}list.yaml",
        MAIN_SEPARATOR, MAIN_SEPARATOR
    );

    let sources_list = src_builder(sources_list_path, &mut buf2, "schemes");
    let templates_list = src_builder(templates_list_path, &mut buf3, "templates");

    vec![sources_list, templates_list]
        .par_iter()
        .for_each(|srcs| {
            src_getter(srcs);
        });
}

/// Gets the provided sources asynchronously
fn src_getter(srcs: &Vec<Source>) {
    srcs.par_iter().for_each(|src| {
        let Source { repo, source, kind } = src;
        git_clone(
            repo.to_string(),
            format!("{}{}{}",kind, MAIN_SEPARATOR, source),
        );
    });
}

/// Create a source vector from a given path to a yml file
fn src_builder<'a, 'b>(src: &'a str, buf: &'b mut Yaml, kind: &'b str) -> Vec<Source<'b>> {
    match fs::metadata(&src) {
        Ok(_) => {}
        Err(_) => {
            error!("{} not found", src);
            exit(1);
        }
    };

    *buf = read_yaml_file(src).remove(0);
    let sources: Vec<Source> = buf
        .as_hash()
        .unwrap()
        .iter()
        .map(|(source, repo)| Source {
            source: source.as_str().unwrap(),
            repo: repo.as_str().unwrap(),
            kind: kind,
        })
        .collect();

    sources
}

/// Takes a maybe theme and if it is a theme,
/// only builds that theme, if not, builds all themes
fn build_themes(theme: Option<&str>) {
    let templates = get_templates();
    let mut schemes = get_schemes();

    match theme {
        Some(theme_name) => {
            schemes.drain_filter(|thm| thm.name.to_lowercase() != theme_name.to_lowercase());
        }
        _ => (),
    }

    schemes.par_iter().for_each(|scheme| {
        for template in &templates {
            info!(
                "Building {}/base16-{}{}",
                template.output,
                scheme.slug.to_string(),
                template.extension
            );
            let mut data = HashBuilder::new();
            data = data.insert("scheme-slug", scheme.slug.as_ref());
            data = data.insert("scheme-name", scheme.name.as_ref());
            data = data.insert("scheme-author", scheme.author.as_ref());

            for (base, color) in &scheme.colors {
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

            let _ = fs::create_dir(format!("{}", template.output));
            let filename = format!(
                "{}{}base16-{}{}",
                template.output,
                MAIN_SEPARATOR,
                scheme.slug.to_lowercase().replace(" ", "_"),
                template.extension
            );
            let f = match File::create(&filename) {
                Ok(result) => result,
                Err(err) => {
                    error!("Failed to create {} with \"{}\"", &filename, err);
                    continue;
                }
            };
            let mut out = BufWriter::new(f);
            match data.render(&template.data, &mut out) {
                Ok(_) => (),
                Err(_) => {
                    error!(
                        "Data for \"{}-{}\" could not be renderd",
                        &scheme.slug, &template.extension
                    );
                    exit(2);
                }
            };
            println!("Built base16-{}{}", &scheme.slug, &template.extension);
        }
    });
}

/// Goes into the templates dir and for each directory
/// extracts the templates
fn get_templates() -> Vec<Template> {
    let mut templates = vec![];

    // Find the templates inside the template dir
    for template_dir in
        fs::read_dir("templates").expect("Could not read into the `templates` directory")
    {
        let template_dir = match template_dir {
            Ok(dir) => dir,
            Err(e) => {
                error!("Could not access template dir, error: {}", e);
                exit(2);
            }
        }
        .path();
        let template_dir_path = template_dir
            .to_str()
            .expect("Could not cast template_dir into str");
        let template_config = &read_yaml_file(&format!(
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

/// Reads the schemes directory and parses all the
/// themes returning a vector with the parsed structures
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

                        let slug = &read_yaml_file(&scheme_file.to_string_lossy())[0];
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

/// Self explanatory
fn read_yaml_file(file: &str) -> Vec<yaml_rust::Yaml> {
    debug!("Reading YAML file {}", file);
    let mut src_file = File::open(file).unwrap();
    let mut srcs = String::new();
    src_file.read_to_string(&mut srcs).unwrap();

    YamlLoader::load_from_str(&mut srcs).unwrap()
}

/// Updates or clones a given repository into a given path
fn git_clone(url: String, path: String) {
    println!("-- {}", &path);
    match fs::metadata(&path) {
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
