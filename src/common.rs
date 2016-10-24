use ::types::*;
use ::graph::CachedGraph;


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

    pub fn child(mut self, child: TypeTree) -> TypeTree
    {
        self.children.push(child);
        self
    }
}

impl<'a> NodeTree<'a> {
    pub fn new(node: &'a Node, label: &'a str, correlation: Correlation) -> NodeTree<'a>
    {
        NodeTree {
            node: node,
            title: label,
            correlation: correlation,
            children: Vec::new(),
        }
    }

    pub fn print(&self, level: u8)
    {
        for _ in 0..level + 1 { print!("|--") }
        println!(" {}", self.node);
        for child in &self.children {
            child.print(level+1)
        }
    }

    pub fn child(mut self, child: NodeTree<'a>) -> NodeTree
    {
        self.children.push(child);
        self
    }

    pub fn construct(graph: &'a CachedGraph, type_tree: &'a TypeTree, root: &'a Node) -> NodeTree<'a>
    {
        let mut tree = NodeTree::new(root, &*type_tree.label, type_tree.correlation.clone());
        for child_type in &type_tree.children {
            let neighbors = graph.neighbors_labeled(&root.id, &child_type.title);
            for neighbor in neighbors {
                tree = tree.child(NodeTree::construct(graph, child_type, neighbor))
            }
        }
        tree
    }

    pub fn flatten(&self) -> Vec<&'a Node>
    {
        let mut nodes = vec![self.node];
        for child in &self.children {
            nodes.append(&mut child.flatten())
        }
        nodes
    }
}

pub fn sample_type_tree() -> TypeTree
{
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


pub fn case_type_tree() -> TypeTree
{
    TypeTree::new("case", "cases", Correlation::ToMany)
        .child(sample_type_tree())
        .child(TypeTree::new("annotation", "annotations", Correlation::ToMany))
        .child(TypeTree::new("project", "project", Correlation::ToOne))
        .child(TypeTree::new("program", "program", Correlation::ToOne)
               .child(TypeTree::new("program", "program", Correlation::ToOne)))
        .child(TypeTree::new("file", "files", Correlation::ToMany))
        .child(TypeTree::new("tissue_source_site", "tissue_source_site", Correlation::ToOne))
        .child(sample_type_tree())
        .child(TypeTree::new("demographic", "demographic", Correlation::ToOne))
        .child(TypeTree::new("exposure", "exposures", Correlation::ToMany))
        .child(TypeTree::new("diagnosis", "diagnoses", Correlation::ToMany)
               .child(TypeTree::new("treatment", "treatments", Correlation::ToMany)))
        .child(TypeTree::new("family_history", "family_history", Correlation::ToMany))
}


pub fn denormalize_tree(options: &CachingOptions, graph: &CachedGraph, tree: &NodeTree) -> Doc
{
    let mut doc = tree.node.get_base_doc();
    for child in &tree.children {
        setitem!(doc, child.title.to_string(), denormalize_tree(options, graph, child))
    }
    doc
}
