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
use postgres::error::ConnectError;
use postgres::{Connection, SslMode};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
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

            // Add the edge in both directions
            self.graph.get_mut(&edge.src_id).unwrap()
                .get_mut(&edge.dst_id).unwrap_or(&mut Vec::new()).push(edge.clone());
            self.graph.get_mut(&edge.dst_id).unwrap()
                .get_mut(&edge.src_id).unwrap_or(&mut Vec::new()).push(edge.clone());

            Ok(())
        }
    }

    /// Returns a reference to a node given it's id
    pub fn get_node<'a>(&'a self, id: &String) -> Option<&'a Node>
    {
        self.nodes.get(id)
    }

    /// Returns a vector of all node references with a given label
    pub fn nodes_labeled<'a, S>(&'a self, label: S) -> Vec<&'a Node>
        where S: Into<String>
    {
        let label: String = label.into();
        self.nodes.values().filter(|node| node.label == label).collect()
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
    pub fn neighbors_labeled<'a>(&'a self, id: &String, label: &String) -> Vec<&'a Node>
    {
        // edges should be bidirectional, just pick one direction
        match self.graph.get(id) {
            Some(map) => {
                map.iter().map(|(dst, _)| self.get_node(dst).unwrap())
                    .filter(|node| &node.label == label)
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

    /// Loads all Node and Edge tables defined in the datamodel using the
    /// given Postgres connection
    #[allow(unused_variables)]
    pub fn from_postgres(options: &CachingOptions, datamodel: &Datamodel, connection: &Connection)
                         -> EBResult<CachedGraph>
    {
        let mut graph = CachedGraph::new();

        // Load all nodes first
        for (_, node_type) in &datamodel.node_types {
            let nodes: Vec<Node> = load_node_table(node_type, &connection)?;
            for node in nodes {
                graph.add_node(node);
            }
        }

        // Load all edges to point to already loaded nodes
        for (_, node_type) in &datamodel.node_types {
            for link in &node_type.links {
                let edges = load_edge_table(link, &connection)?;
                for edge in edges {
                    graph.add_edge(edge)?;
                }
            }
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
