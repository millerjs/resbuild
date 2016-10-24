use serde_json::Value;
use std::collections::HashMap;
use serde_json::Map;
use regex::Regex;

pub type Doc = Map<String, Value>;

#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub props: Doc,
    pub sysan: Doc,
    pub acl: Vec<String>,
}


#[derive(Debug)]
pub enum NodeCategory {
    DataFile,
    Biospecimen,
    Notation,
    Administrative,
    Analysis,
    Clinical,
    IndexFile,
    MetadataFile,
    Other,
}

#[derive(Debug)]
pub struct NodeType {
    pub label: String,
    pub category: NodeCategory,
    pub links: Vec<EdgeType>,
    pub backrefs: Vec<EdgeType>,
}

#[derive(Debug,Clone)]
pub struct Edge {
    pub src_id: String,
    pub dst_id: String,
    pub label: String,
}

#[derive(Debug)]
pub struct EdgeType {
    pub name: String,
    pub label: String,
    pub backref: String,
    pub src_label: String,
    pub dst_label: String,
}

#[derive(Debug,Clone)]
pub enum Correlation {
    ToOne,
    ToMany,
}

#[derive(Debug)]
pub struct CachingOptions {
    pub case_to_file_paths: Vec<Vec<String>>,
    pub redacted_but_not_suppressed: Vec<String>,
    pub differentiated_edges: Vec<(String, String, String)>,
    pub file_labels: Vec<String>,
    pub unindexed_by_property: HashMap<String, Vec<Doc>>,
    pub omitted_projects: Vec<String>,
    pub index_file_extensions: Vec<String>,
    pub possible_associated_entites: Vec<String>,
    pub supplement_regexes: Vec<Regex>,
}

#[derive(Debug)]
pub struct TypeTree {
    pub label: String,
    pub title: String,
    pub correlation: Correlation,
    pub children: Vec<TypeTree>,
}

#[derive(Debug)]
pub struct NodeTree<'a> {
    pub node: &'a Node,
    pub title: &'a str,
    pub correlation: Correlation,
    pub children: Vec<NodeTree<'a>>,
}

#[derive(Debug)]
pub struct Datamodel {
    pub node_types: HashMap<String, NodeType>,
}
