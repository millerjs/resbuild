use ::dictionary::SCHEMAS;
use ::edge::{EdgeType};
use ::node::{NodeCategory, NodeType};
use ::errors::{EBResult, EBError};
use std::collections::{HashMap, HashSet, BTreeMap};
use yaml_rust::{YamlLoader, Yaml};


#[derive(Debug)]
pub struct Datamodel {
    pub node_types: HashMap<String, NodeType>,
}

#[derive(Debug)]
pub struct SchemaNode {
    pub key: String,
    pub value: Option<String>,
    pub children: Vec<SchemaNode>,
}


#[derive(Debug)]
pub enum PropertyType {
    Integer,
    Decimal,
    String,
    Boolean,
}


impl Datamodel {
    pub fn new() -> EBResult<Datamodel> {
        let mut node_types = HashMap::new();
        let resolver = &Resolver::new()?;

        for schema in SCHEMAS.iter() {
            let yaml = load_yaml(schema.as_ref())?;
            let id = yaml_str(&yaml, "id")?;

            if id.starts_with("_") {
                debug!("Skipping schema {:?}", id);
                continue
            }

            let resolved = resolver.resolve("root", &yaml)?;
            let node_type = resolved.node_type()?;

            debug!("Loaded schema {}", node_type.label);
            node_types.insert(node_type.label.clone(), node_type);
        }

        Ok(Datamodel { node_types: node_types })
    }
}


impl<'a> From<&'a str> for NodeCategory {
    fn from(category: &str)-> NodeCategory {
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
}


impl PropertyType {
    fn parse(type_str: &str) -> EBResult<PropertyType> {
        Ok(match &*type_str.to_lowercase() {
            "bool" => PropertyType::Boolean,
            "boolean" => PropertyType::Boolean,
            "datetime" => PropertyType::String,
            "enum" => PropertyType::String,
            "float" => PropertyType::Decimal,
            "integer" => PropertyType::Decimal,
            "number" => PropertyType::Decimal,
            "string" => PropertyType::String,
            _ => return Err(format!("Unknown type: {}", type_str).into()),
        })
    }
}

impl SchemaNode {
    fn new<S>(key: S) -> SchemaNode where S: Into<String> {
        SchemaNode { key: key.into(), value: None, children: Vec::new() }
    }

    fn get_kv(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|node| node.value.clone())
    }

    fn print(&self, level: u8) {
        for _ in 0..level + 1 { print!("|--") }
        println!(" {}: {:?}", self.key, self.value);
        for child in &self.children {
            child.print(level+1)
        }
    }

    fn get<'a>(&'a self, key: &str) -> Option<&'a SchemaNode> {
        self.children.iter().find(|child| child.key == key)
    }

    fn node_properties(&self, links: &Vec<EdgeType>) -> EBResult<HashMap<String, PropertyType>> {
        let mut props = HashMap::new();
        let props_node = self.get("properties").ok_or("missing properties")?;
        let link_names = links.iter().map(|l| l.name.clone()).collect::<HashSet<_>>();
        let prop_nodes = props_node.children.iter().filter(|node| !link_names.contains(&node.key));
        for prop_node in prop_nodes {
            let prop_type = prop_node.get("type")
                .and_then(|n| n.value.clone())
                .map(|type_str| PropertyType::parse(&*type_str))
                .unwrap_or(Ok(PropertyType::String));
            props.insert(prop_node.key.clone(), prop_type?);
        }
        Ok(props)
    }

    fn node_type(&self) -> EBResult<NodeType> {
        let label = self.get_kv("id").ok_or("missing label")?;
        let category_str = &*self.get_kv("category").ok_or("missing category")?;

        let links = self.edge_types(&label)?;
        let backrefs = links.iter().map(|ref link| EdgeType {
            src_label: link.dst_label.clone(),
            dst_label: link.src_label.clone(),
            name: link.backref.clone(),
            backref: link.name.clone(),
            label: link.label.clone(),
        }).collect();

        let properties = self.node_properties(&links)?;

        Ok(NodeType {
            label: label,
            props: properties,
            category: category_str.into(),
            links: links,
            backrefs: backrefs,
        })
    }

    fn edge_type(&self, src_label: &String) -> EBResult<EdgeType> {
        let dst_label = self.get_kv("target_type").ok_or(format!("{:?} missing target", self))?;
        let backref = self.get_kv("backref").ok_or(format!("{:?} missing backref", self))?;
        let name = self.get_kv("name").ok_or(format!("{:?} missing name", self))?;
        let label = self.get_kv("label").ok_or(format!("{:?} missing label", self))?;

        Ok(EdgeType {
            src_label: src_label.clone(),
            dst_label: dst_label.to_string(),
            backref: backref.to_string(),
            name: name.to_string(),
            label: label.to_string()
        })
    }

    fn edge_types(&self, src_label: &String) -> EBResult<Vec<EdgeType>> {
        if !self.get("links").is_some() {
            return Ok(vec![])
        }

        let mut edges = Vec::new();
        let links = self.get("links").ok_or("links must be a list")?;

        for entry in &links.children {
            if let Some(subgroup) = entry.get("subgroup") {
                for link in &subgroup.children {
                    edges.push(link.edge_type(src_label)?);
                }
            } else {
                edges.push(entry.edge_type(src_label)?);
            }
        }

        Ok(edges)
    }
}

pub struct Resolver {
    schemas: HashMap<String, Yaml>,
}


pub fn yaml_str<'a>(yaml: &'a Yaml, key: &str) -> EBResult<&'a str> {
    match yaml[key].as_str() {
        Some(key_str) => Ok(key_str),
        None => return Err(EBError::BuildError(
            format!("unable to parse key '{:}' to string in {:?}", key, yaml))),
    }
}


pub fn load_yaml(source: &str) -> EBResult<Yaml> {
    Ok(YamlLoader::load_from_str(source)?[0].clone())
}


impl Resolver {

    fn new() -> EBResult<Resolver> {
        let mut schemas = HashMap::new();
        for schema in SCHEMAS.iter() {
            let schema = load_yaml(&*schema)?;
            let label = yaml_str(&schema, "id")?.to_string();
            schemas.insert(label, schema);
        }
        Ok(Resolver { schemas: schemas })
    }

    fn dereference<'a>(&'a self, referrer: &'a Yaml, identifier: &str) -> EBResult<&'a Yaml> {
        let parts = identifier.split("#/").collect::<Vec<&str>>();
        let root = parts.get(0).ok_or(format!("Unable to parse id from $ref {}", identifier))?;
        let path = parts.get(1).ok_or(format!("Unable to parse path from $ref {}", identifier))?;
        let id = root.split(".").collect::<Vec<_>>()[0];
        let schema = match id {
            "" => referrer,
            _ => self.schemas.get(id).ok_or(format!("missing schema: {:?}", id))?,
        };
        let resolution = &schema[*path];
        Ok(resolution)
    }

    fn resolve_hash(&self, key: &str, hash: &BTreeMap<Yaml, Yaml>) -> EBResult<Vec<SchemaNode>> {
        let mut nodes = Vec::new();
        for (child_key, child_schema) in hash {
            let child_key = child_key.as_str().ok_or("unable to parse string")?;

            if "$ref" == child_key {
                let child_value = child_schema.as_str().ok_or("ref not a string")?;
                let deref = self.dereference(child_schema, child_value)?;
                let children = self.resolve(key, deref)?.children;
                for child in children {
                    nodes.push(child)
                }

            } else {
                let child = self.resolve(child_key, child_schema)?;
                nodes.push(child);
            }
        }
        Ok(nodes)
    }

    fn resolve(&self, key: &str, schema: &Yaml) -> EBResult<SchemaNode> {
        let mut node = SchemaNode::new(key);

        // If yaml is a hash
        if let Some(hash) = schema.as_hash() {
            for child in self.resolve_hash(key, hash)? {
                node.children.push(child)
            }

        // If yaml is a list
        } else if let Some(vec) = schema.as_vec() {
            for child_schema in vec {
                node.children.push(self.resolve(key, child_schema)?);
            }

        // Otherwise just save the value
        } else {
            node.value = schema.as_str().map(|s| s.to_string())
        }

        Ok(node)
    }
}
