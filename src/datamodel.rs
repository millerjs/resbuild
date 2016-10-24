use std::collections::HashMap;

use ::types::*;
use ::errors::EBResult;
use walkdir::WalkDir;
use std::io::prelude::*;
use std::fs::File;
use yaml_rust::{YamlLoader, Yaml};


fn get_node_category(category: &str) -> NodeCategory
{
    match category {
        "data_file" => NodeCategory::DataFile,
        "biospecimen" => NodeCategory::Biospecimen,
        "notation" => NodeCategory::Notation,
        "administrative" => NodeCategory::Administrative,
        "analysis" => NodeCategory::Analysis,
        "clinical" => NodeCategory::Clinical,
        "index_file" => NodeCategory::IndexFile,
        "metadata_file" => NodeCategory::MetadataFile,
        _ => NodeCategory::Other,
    }
}


fn parse_node_type_from_schema(schema: &String) -> EBResult<NodeType>
{
    let yaml = &try!(YamlLoader::load_from_str(&*schema))[0];

    let label = try!(yaml["id"].as_str().ok_or("id must be a string")).to_string();
    let category_str = try!(yaml["id"].as_str().ok_or("category must be a string"));
    let category = get_node_category(category_str);

    let links = try!(parse_edge_types_from_schema(&label, schema));
    let backrefs = links.iter().map(|ref link| EdgeType {
        src_label: link.dst_label.clone(),
        dst_label: link.src_label.clone(),
        name: link.backref.clone(),
        backref: link.name.clone(),
        label: link.label.clone(),
    }).collect();

    Ok(NodeType {
        label: label,
        category: category,
        links: links,
        backrefs: backrefs,
    })
}


fn parse_edge_type_from_yaml(src_label: &String, yaml: &Yaml) -> EBResult<EdgeType>
{
    let dst_label = try!(yaml["target_type"].as_str().ok_or(
        format!("Link {:?} missing target", yaml)));
    let backref = try!(yaml["backref"].as_str().ok_or(
        format!("Link {:?} missing backref", yaml)));
    let name = try!(yaml["name"].as_str().ok_or(
        format!("Link {:?} missing name", yaml)));
    let label = try!(yaml["label"].as_str().ok_or(
        format!("Link {:?} missing label", yaml)));

    Ok(EdgeType {
        src_label: src_label.clone(),
        dst_label: dst_label.to_string(),
        backref: backref.to_string(),
        name: name.to_string(),
        label: label.to_string()
    })
}


fn parse_edge_types_from_schema(src_label: &String, schema: &String) -> EBResult<Vec<EdgeType>>
{
    let yaml = &try!(YamlLoader::load_from_str(&*schema))[0];

    if yaml["links"].is_badvalue() {
        return Ok(vec![])
    }

    let mut edges = Vec::new();
    for entry in try!(yaml["links"].as_vec().ok_or("links must be a list")) {
        if !entry["subgroup"].is_badvalue() {
            for link in entry["subgroup"].as_vec().unwrap() {
                edges.push(try!(parse_edge_type_from_yaml(src_label, link)))
            }
        } else {
            edges.push(try!(parse_edge_type_from_yaml(src_label, entry)))
        };
    }

    Ok(edges)
}


impl Datamodel {
    pub fn new() -> Datamodel
    {
        Datamodel {
            node_types: HashMap::new(),
        }
    }

    pub fn load_from_dictionary(mut self, root_path: &str) -> EBResult<Datamodel>
    {
        info!("Loading dictionary from {}", root_path);

        let entries = WalkDir::new(root_path).into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().file_name().is_some())
            .filter(|e| e.path().file_name().unwrap().to_str().is_some())
            .filter(|e| format!("{}", e.path().display()).ends_with(".yaml"))
            .filter(|e| !format!("{}", e.path().display()).contains("metaschema.yaml"))
            .filter(|e| !format!("{}", e.path().display()).contains("_terms.yaml"))
            .filter(|e| !format!("{}", e.path().display()).contains("_definitions.yaml"));

        for entry in entries {
            debug!("Loading schema {:?}", entry.path());
            let mut file = try!(File::open(entry.path()));
            let mut contents = String::new();
            try!(file.read_to_string(&mut contents));
            let node_type = try!(parse_node_type_from_schema(&contents));
            self.node_types.insert(node_type.label.clone(), node_type);
        }

        Ok(self)
    }
}
