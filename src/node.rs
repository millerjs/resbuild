use std::fmt;
use ::types::*;
use serde_json::Value;


impl Node {
    pub fn new(label: String, id: String, props: Doc, sysan: Doc, acl: Vec<String>) -> Node
    {
        Node { label: label, id: id, props: props, sysan: sysan, acl: acl }
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn category(&self) -> NodeCategory
    {
        NodeCategory::Biospecimen
    }

    /// This is the basic document generator.  Take all the properties
    /// of a node and add it the the result.
    #[inline(always)]
    pub fn get_base_doc_without_id(&self, options: &Options) -> Doc
    {
        let mut doc = Doc::new();
        options.datamodel.node_types
            .get(&self.label).unwrap().props.iter()
            .filter(|&(key, _)| !self.is_prop_hidden(&*key))
            .map(|(key, _)| setitem!(doc, key, *self.props.get(key).unwrap_or(&Value::Null)))
            .collect::<Vec<()>>();
        doc
    }

    /// Returns a boolean wether the given key should be included in
    /// the base doc for this node
    #[inline(always)]
    pub fn is_prop_hidden(&self, key: &str) -> bool
    {
        (key == "project_id" && &*self.label != "project")
    }

    #[inline(always)]
    /// The result doc will have *_id where * is the node type.
    pub fn get_base_doc(&self, options: &Options) -> Doc
    {
        let mut doc = self.get_base_doc_without_id(options);

        let id_key = match self.category() {
            NodeCategory::Analysis => "analysis_id".into(),
            _ => format!("{}_id", self.label),
        };

        setitem!(doc, id_key, self.id);
        doc
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
