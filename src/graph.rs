//! Graph
//!
//! This module provides the ability to cache the GDC graph model in
//! memory.

use ::errors::*;
use ::edge::{Edge, EdgeType};
use ::node::{Node, NodeType};
use ::types::Doc;
use ::datamodel::Datamodel;
use openssl;
use pbr::ProgressBar;
use postgres::error::ConnectError;
use postgres::{Connection, SslMode};
use serde_json::Value;
use std::collections::{HashSet, HashMap};
use std::fmt::Display;
use std::hash::Hash;
use regex::Regex;


/// CachingOptions contains the specifics of what to load from the
/// graph, what to drop, and what to specifically cache
/// (e.g. relationships).
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

/// CachedGraph is an in memory representation of the GDC graph as
/// represented by a hash (id->Node) and a `graph` hash
/// (src_id->dst_ids->Edge).
#[derive(Debug)]
pub struct CachedGraph {
    pub graph: HashMap<String, HashMap<String, Vec<Edge>>>,
    pub nodes: HashMap<String, Node>
}


impl CachingOptions {
    /// Creates a new CachingOption with defaults (mostly empty) settings
    pub fn new() -> CachingOptions {
        CachingOptions {
            case_to_file_paths: Vec::new(),
            redacted_but_not_suppressed: Vec::new(),
            differentiated_edges: Vec::new(),
            file_labels: Vec::new(),
            unindexed_by_property: HashMap::new(),
            omitted_projects: Vec::new(),
            index_file_extensions: Vec::new(),
            possible_associated_entites: Vec::new(),
            supplement_regexes: Vec::new(),
        }
    }
}


/// Turn a vec of hashable items into a set
fn vec_to_set<T>(source: &Vec<T>) -> HashSet<T>
    where T: Hash + Eq + Clone
{
    let mut ret = HashSet::<T>::new();
    for item in source {
        ret.insert(item.clone());
    }
    ret
}


impl CachedGraph {
    /// Creates a new empty CachedGraph.  To create one by importing
    /// from postgres, prefer `CachedGraph::from_postgres(...)`
    pub fn new() -> CachedGraph
    {
        CachedGraph {
            nodes: HashMap::new(),
            graph: HashMap::new()
        }
    }

    /// Adds the edge to the graph in both directions. This is for
    /// ease of lookup later.  If we only cached in one direction,
    /// then we would have to do up to two lookups from the graph to
    /// find an edge.
    pub fn add_edge(&mut self, edge: Edge) -> EBResult<()>
    {
        if !self.nodes.contains_key(&edge.src_id) {
            Err(format!("Source id {} not in graph", edge.src_id).into())
        } else if !self.nodes.contains_key(&edge.dst_id) {
            Err(format!("Destination id {} not in graph", edge.dst_id).into())
        } else {
            // Create a src -> dst map if it doesn't exist
            if !self.graph.contains_key(&edge.src_id) {
                self.graph.insert(edge.src_id.clone(), HashMap::new());
            }

            // Create a dst -> src map if it doesn't exist
            if !self.graph.contains_key(&edge.dst_id) {
                self.graph.insert(edge.dst_id.clone(), HashMap::new());
            }

            // Create a src -> dst edge list if it doesn't exist
            if !self.graph.get(&edge.src_id).unwrap().contains_key(&edge.dst_id) {
                self.graph.get_mut(&edge.src_id).unwrap().insert(edge.dst_id.clone(), vec![]);
            }

            // Create a dst -> src edge list if it doesn't exist
            if !self.graph.get(&edge.dst_id).unwrap().contains_key(&edge.src_id) {
                self.graph.get_mut(&edge.dst_id).unwrap().insert(edge.src_id.clone(), vec![]);
            }

            self.graph.get_mut(&edge.src_id).unwrap().get_mut(&edge.dst_id).unwrap()
                .push(edge.clone());
            self.graph.get_mut(&edge.dst_id).unwrap().get_mut(&edge.src_id).unwrap()
                .push(edge.clone());

            Ok(())
        }
    }

    /// Returns a reference to a node given it's id
    pub fn get_node<'a>(&'a self, id: &String) -> Option<&'a Node>
    {
        self.nodes.get(id)
    }

    /// Returns a vector of all node references with a given label
    pub fn nodes_labeled<'a>(&'a self, labels_: &Vec<String>) -> Vec<&'a Node>
    {
        let labels = vec_to_set(labels_);
        self.nodes.values().filter(|node| labels.contains(&node.label)).collect()
    }

    /// Returns a vec of node references that are adjacent to the node
    /// with the given `id`
    pub fn neighbors<'a>(&'a self, id: &String) -> Vec<&'a Node>
    {
        // edges should be bidirectional, just pick one direction
        match self.graph.get(id) {
            Some(map) => {
                map.iter().map(|(dst, _)| self.get_node(dst).unwrap()).collect()
            },
            None => Vec::new(),
        }
    }

    /// Returns a vec of node references that are adjacent to the node
    /// with the given `id` and have a given `label`
    pub fn neighbors_labeled<'a>(&'a self, id: &String, labels_: &Vec<String>) -> Vec<&'a Node>
    {
        let labels = vec_to_set(labels_);
        // edges should be bidirectional, just pick one direction
        match self.graph.get(id) {
            Some(map) => {
                map.iter().map(|(dst, _)| self.get_node(dst).unwrap())
                    .filter(|node| labels.contains(&node.label))
                    .collect()
            },
            None => Vec::new(),
        }
    }

    /// Returns a reference to any edge between given nodes
    /// (agnostic of directionality)
    pub fn get_edges<'a>(&'a self, src_id: &String, dst_id: &String) -> Option<&Vec<Edge>>
    {
        self.graph.get(src_id).map_or(None, |r| r.get(dst_id))
    }

    /// Idempotently add a node to the map of nodes
    pub fn add_node(&mut self, node: Node)
    {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn walk_path<'a>(&'a self, node_id: &String, path: &[String], whole: bool)
                         -> Vec<&'a Node>
    {
        let mut found = HashSet::<String>::new();
        let mut nodes = Vec::<&Node>::new();

        if path.len() == 0 {
            return vec![]
        }

        let neighbors = self.neighbors_labeled(node_id, &vec![path[0].clone()]);

        for neighbor in neighbors {
            for node in self.walk_path(&neighbor.id, &path[1..], whole) {
                if !found.contains(&node.id.clone()) {
                    found.insert(node.id.clone());
                    nodes.push(node);
                }
            }
            if whole || (path.len() == 1 && neighbor.label == path[0]) {
                if !found.contains(&neighbor.id.clone()) {
                    found.insert(neighbor.id.clone());
                    nodes.push(neighbor);
                }
            }
        };

        nodes
    }

    pub fn walk_paths<'a>(&'a self, node_id: &String, paths: &Vec<Vec<String>>, whole: bool)
                          -> Vec<&'a Node>
    {
        let mut found = HashSet::<String>::new();
        let mut nodes = Vec::<&Node>::new();

        for path in paths {
            for node in self.walk_path(node_id, &path[..], whole) {
                found.insert(node.id.clone());
                nodes.push(node);
            }
        }
        nodes
    }

    /// Remove a node and associated edges from the graph
    pub fn remove_node(&mut self, id: &String) -> Option<Node>
    {
        let neighbor_ids = self.neighbors(id).iter()
            .map(|n| n.id.clone()).collect::<Vec<_>>();

        // Delete edges where node = src
        self.graph.remove(id);

        // Delete edges where node = dst
        for neighbor_id in neighbor_ids {
            if let Some(src) = self.graph.get_mut(&neighbor_id) {
                src.remove(id);
            }
        }

        self.nodes.remove(id)
    }

    /// Loads all Node and Edge tables defined in the datamodel using the
    /// given Postgres connection
    #[allow(unused_variables)]
    pub fn from_postgres(options: &CachingOptions, datamodel: &Datamodel, connection: &Connection)
                         -> EBResult<CachedGraph>
    {
        let mut graph = CachedGraph::new();

        // Load all nodes first
        let mut progress_bar = ProgressBar::new(datamodel.node_types.len() as u64);
        for (_, node_type) in &datamodel.node_types {
            progress_bar.message(&*format!("Loading nodes {:50} ", node_type.label));

            let nodes: Vec<Node> = load_node_table(node_type, &connection)?;
            for node in nodes {
                graph.add_node(node);
            }
            progress_bar.inc();
        }

        // Load all edges to point to already loaded nodes
        let mut progress_bar = ProgressBar::new(datamodel.node_types.len() as u64);
        for (_, node_type) in &datamodel.node_types {
            for link in &node_type.links {

                let edge_name = format!("{:}->{:}->{:}", link.src_label, link.label, link.dst_label);
                progress_bar.message(&*format!("Loading edges {:50} ", edge_name));

                let edges = load_edge_table(link, &connection)?;
                for edge in edges {
                    graph.add_edge(edge)?;
                }
            }
            progress_bar.inc();
        }

        info!("Loaded {} nodes from postgres", graph.nodes.len());
        Ok(graph)
    }
}

/// Returns a connection to Postgres if able to connect
pub fn connect<S>(host: S, database: S, user: S, pass: S) -> Result<Connection, ConnectError>
    where S: Display
{
    let conn_str = format!("postgres://{}:{}@{}/{}", user, pass, host, database);
    let ctx = openssl::ssl::SslContext::new(openssl::ssl::SslMethod::Sslv23).unwrap();
    Connection::connect(&*conn_str,  SslMode::Prefer(&ctx))
}


/// Loads all nodes in the table corresponding to the given NodeType
pub fn load_node_table(node_type: &NodeType, connection: &Connection) -> EBResult<Vec<Node>>
{
    let label = &node_type.label;

    debug!("Loading node type: {}", label);
    let tablename = node_type.get_tablename();
    let statement = format!("SELECT node_id, _props, _sysan, acl FROM {}", tablename);
    let rows = connection.query(&*statement, &[])?;

    debug!("Loaded {} {} nodes", rows.len(), label);
    let mut nodes = Vec::with_capacity(rows.len());

    for row in &rows {
        let id: String = row.get(0);
        let props: Value = row.get(1);
        let sysan: Value = row.get(2);
        let props = props.as_object().ok_or("Props must be an object")?;
        let sysan = sysan.as_object().ok_or("Sysan must be an object")?;
        let acl = row.get(3);
        let node = Node::new(label.clone(), id.clone(), props.clone(), sysan.clone(), acl);
        nodes.push(node);
    }

    debug!("Instantiated {} {} nodes", rows.len(), node_type.label);
    Ok(nodes)
}


/// Loads all edges in the table corresponding to the given EdgeType
pub fn load_edge_table(edge_type: &EdgeType, connection: &Connection) -> EBResult<Vec<Edge>>
{
    let label = &edge_type.label;
    debug!("{}", edge_type.get_tablename());
    debug!("Loading edge type: {}", label);

    let tablename = edge_type.get_tablename();
    let statement = format!("SELECT src_id, dst_id FROM {}", tablename);
    let rows = connection.query(&*statement, &[])?;

    debug!("Loaded {} {} edges", rows.len(), label);
    let mut edges = Vec::with_capacity(rows.len());

    for row in &rows {
        let src_id: String = row.get(0);
        let dst_id: String = row.get(1);
        let edge = Edge::new(label.clone(), src_id, dst_id);
        edges.push(edge);
    }

    debug!("Instantiated {} {} edges", rows.len(), edge_type.label);
    Ok(edges)
}
