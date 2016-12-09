use ::datamodel::Datamodel;
use ::graph::CachedGraph;
use ::node::{Node, NodeCategory};
use ::types::Doc;

use serde_json::Value;

// ======================================================================
// Types

#[derive(Debug,Clone)]
pub enum Correlation {
    ToOne,
    ToMany,
}

#[derive(Debug)]
pub struct Options {
    pub datamodel: Datamodel,
    pub case_to_file_paths: Vec<Vec<String>>,
    pub file_labels: Vec<String>,
    pub index_file_extensions: Vec<String>,
    pub possible_associated_entites: Vec<String>,
}

/// A type tree is a tree structure containing the entities to
/// traverse in the graph at the type level, e.g. a case tree
///
/// case
///  |___ sample
///  |      |___ portion
///  |              |___ analyte
///  |                      ...
///  |___ annotation
///
#[derive(Debug)]
pub struct TypeTree {
    pub label: String,
    pub title: String,
    pub correlation: Correlation,
    pub children: Vec<TypeTree>,
}


/// A node tree is a tree structure containing the entities at the
/// node instance level produced by traversing a TypeTree in the
/// graph, e.g. a case tree
///
/// case1
///  |___ sample1
///  |      |___ portion1
///  |              |___ analyte1
///  |                      ...
///  |___ sample2
///  |      |___ portion2
///  |              |___ analyte2
///  |                      ...
///  |___ annotation1
///
#[derive(Debug)]
pub struct NodeTree<'a> {
    pub node: &'a Node,
    pub title: &'a str,
    pub correlation: Correlation,
    pub children: Vec<NodeTree<'a>>,
}


// ======================================================================
// Implementations

pub trait Builder {
    fn options<'a>(&'a self) -> &'a Options;
    fn graph<'a>(&'a self) -> &'a CachedGraph;

    /// Produce a base document from a given NodeTree (that was
    /// probably produced from a TypeTree)
    fn denormalize_tree(&self, tree: &NodeTree) -> Doc {
        let mut doc = self.get_base_doc(tree.node);
        for child in &tree.children {
            let sub_tree = self.denormalize_tree(child);
            setitem!(doc, child.title.to_string(), sub_tree);
        }
        doc
    }

    /// This is the basic document generator.  Take all the properties
    /// of a node and add it the the result.
    #[inline(always)]
    fn get_base_doc_without_id(&self, node: &Node) -> Doc {
        let mut doc = Doc::new();

        let props = self.options().datamodel.node_types
            .get(&node.label).unwrap()
            .props.iter()
            .filter(|&(key, _)| !self.is_prop_hidden(node, &*key));

        for (key, _) in props {
            setitem!(doc, key, *node.props.get(key).unwrap_or(&Value::Null));
        }
        doc
    }

    /// Returns a boolean wether the given key should be included in
    /// the base doc for this node
    #[inline(always)]
    fn is_prop_hidden(&self, node: &Node, key: &str) -> bool {
        key == "project_id" && &*node.label != "project"
    }

    /// The result doc will have *_id where * is the node type.
    #[inline(always)]
    fn get_base_doc(&self, node: &Node) -> Doc {
        let id_key = match node.category() {
            NodeCategory::Analysis => "analysis_id".into(),
            _ => format!("{}_id", node.label),
        };

        let mut doc = self.get_base_doc_without_id(node);
        setitem!(doc, id_key, node.id);
        doc
    }
}


impl TypeTree {
    pub fn new<S>(label: S, title: S, correlation: Correlation) -> TypeTree
        where S: Into<String>
    {
        TypeTree {
            label: label.into(),
            title: title.into(),
            correlation: correlation,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: TypeTree) -> TypeTree {
        self.children.push(child);
        self
    }
}


impl<'a> NodeTree<'a> {
    pub fn new(node: &'a Node, label: &'a str, correlation: Correlation) -> NodeTree<'a> {
        NodeTree {
            node: node,
            title: label,
            correlation: correlation,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: NodeTree<'a>) -> NodeTree {
        self.children.push(child);
        self
    }

    pub fn construct(graph: &'a CachedGraph, type_tree: &'a TypeTree, node: &'a Node) -> NodeTree<'a> {
        let mut tree = NodeTree::new(node, &*type_tree.label, type_tree.correlation.clone());
        for child_type in &type_tree.children {
            let neighbors = graph.neighbors_labeled(&node.id, &child_type.label);
            for neighbor in neighbors {
                tree = tree.child(NodeTree::construct(graph, child_type, neighbor))
            }
        }
        tree
    }

    pub fn flatten(&self) -> Vec<&'a Node> {
        let mut nodes = vec![self.node];
        for child in &self.children {
            nodes.append(&mut child.flatten())
        }
        nodes
    }
}


pub fn sample_type_tree() -> TypeTree {
    TypeTree::new("sample", "samples", Correlation::ToMany)
        .child(TypeTree::new("annotation", "annotations", Correlation::ToMany))
        .child(TypeTree::new("aliquot", "aliquots", Correlation::ToMany))
        .child(TypeTree::new("portion", "portions", Correlation::ToMany)
               .child(TypeTree::new("annotation", "annotations", Correlation::ToMany))
               .child(TypeTree::new("analyte", "analytes", Correlation::ToMany)
                      .child(TypeTree::new("annotation", "annotations", Correlation::ToMany))
                      .child(TypeTree::new("aliquot", "aliquot", Correlation::ToMany)
                             .child(TypeTree::new("annotation", "annotations", Correlation::ToMany)))
                             .child(TypeTree::new("center", "center", Correlation::ToOne)))
               .child(TypeTree::new("slide", "slides", Correlation::ToMany)
                      .child(TypeTree::new("annotation", "annotations", Correlation::ToMany))))
}


pub fn file_type_tree() -> TypeTree {
        TypeTree::new("file", "files", Correlation::ToMany)
        .child(TypeTree::new("annotation", "annotations", Correlation::ToOne))
        .child(TypeTree::new("archive", "archive", Correlation::ToOne))
        .child(TypeTree::new("center", "center", Correlation::ToOne))
        .child(TypeTree::new("data_format", "data_format", Correlation::ToOne))
        .child(TypeTree::new("data_subtype", "data_type", Correlation::ToOne)
               .child(TypeTree::new("data_type", "data_category", Correlation::ToOne)))
        .child(TypeTree::new("experimental_strategy", "experimental_strategy", Correlation::ToOne))
        .child(TypeTree::new("case", "cases", Correlation::ToMany))
        .child(TypeTree::new("platform", "platform", Correlation::ToOne))
        .child(TypeTree::new("tag", "tags", Correlation::ToMany))
        .child(TypeTree::new("file", "metadata_files", Correlation::ToMany))
}


pub fn case_type_tree() -> TypeTree {
    TypeTree::new("case", "cases", Correlation::ToMany)
        .child(sample_type_tree())
        .child(TypeTree::new("annotation", "annotations", Correlation::ToMany))
        .child(TypeTree::new("project", "project", Correlation::ToOne))
        .child(TypeTree::new("program", "program", Correlation::ToOne)
               .child(TypeTree::new("program", "program", Correlation::ToOne)))
        .child(TypeTree::new("file", "files", Correlation::ToMany))
        .child(TypeTree::new("tissue_source_site", "tissue_source_site", Correlation::ToOne))
        .child(sample_type_tree().child(file_type_tree()))
        .child(TypeTree::new("demographic", "demographic", Correlation::ToOne))
        .child(TypeTree::new("exposure", "exposures", Correlation::ToMany))
        .child(TypeTree::new("diagnosis", "diagnoses", Correlation::ToMany)
               .child(TypeTree::new("treatment", "treatments", Correlation::ToMany)))
        .child(TypeTree::new("family_history", "family_history", Correlation::ToMany))
}
