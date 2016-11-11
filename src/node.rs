use ::datamodel::PropertyType;
use ::types::Doc;
use ::edge::EdgeType;
use std::collections::HashMap;
use std::fmt;


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
    pub props: HashMap<String, PropertyType>,
    pub category: NodeCategory,
    pub links: Vec<EdgeType>,
    pub backrefs: Vec<EdgeType>,
}


impl Node {
    pub fn new(label: String, id: String, props: Doc, sysan: Doc, acl: Vec<String>) -> Node {
        Node { label: label, id: id, props: props, sysan: sysan, acl: acl }
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn category(&self) -> NodeCategory
    {
        NodeCategory::Biospecimen
    }
}


impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}({})>", self.label, self.id)
    }
}


impl NodeType {
    pub fn get_tablename(&self) -> String
    {
        format!("node_{}", self.label.replace("_", ""))
    }
}
