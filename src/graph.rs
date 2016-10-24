use openssl;
use postgres::error::ConnectError;
use postgres::{Connection, SslMode};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;

use ::types::*;
use ::errors::*;


#[derive(Debug)]
pub struct CachedGraph {
    pub graph: HashMap<String, HashMap<String, Edge>>,
    pub nodes: HashMap<String, Node>
}


impl CachingOptions {
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

    pub fn new() -> CachedGraph
    {
        CachedGraph {
            nodes: HashMap::new(),
            graph: HashMap::new()
        }
    }

    /// Adds the edge to the graph in both directions
    pub fn add_edge(&mut self, edge: Edge) -> EBResult<()>
    {
        if ! self.nodes.contains_key(&edge.src_id) {
            Err(format!("Source id {} not in graph", edge.src_id).into())
        } else if ! self.nodes.contains_key(&edge.dst_id) {
            Err(format!("Source id {} not in graph", edge.src_id).into())
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
            self.graph.get_mut(&edge.src_id).unwrap().insert(edge.dst_id.clone(), edge.clone());
            self.graph.get_mut(&edge.dst_id).unwrap().insert(edge.src_id.clone(), edge);

            Ok(())
        }
    }

    pub fn get_node<'a>(&'a self, id: &String) -> Option<&'a Node>
    {
        self.nodes.get(id)
    }

    pub fn nodes_labeled<'a, S>(&'a self, label: S) -> Vec<&'a Node>
        where S: Into<String>
    {
        let label: String = label.into();
        self.nodes.values().filter(|node| node.label == label).collect()
    }

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

    pub fn get_edge<'a>(&'a self, src_id: &String, dst_id: &String) -> Option<&'a Edge>
    {
        self.graph.get(src_id).map_or(None, |r| r.get(dst_id))
    }

    pub fn add_node(&mut self, node: Node)
    {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Loads all Node and Edge tables defined in the datamodel using the
    /// given Postgres connection
    pub fn from_postgres(datamodel: &Datamodel, connection: &Connection) -> EBResult<CachedGraph>
    {
        let mut graph = CachedGraph::new();

        for (_, node_type) in &datamodel.node_types {
            let nodes: Vec<Node> = try!(load_node_table(node_type, &connection));
            for node in nodes {
                graph.add_node(node);
            }
        }

        for (_, node_type) in &datamodel.node_types {
            for link in &node_type.links {
                let edges = try!(load_edge_table(link, &connection));

                let filtered_edges = edges.into_iter()
                    .filter(|e| graph.nodes.contains_key(&e.src_id))
                    .filter(|e| graph.nodes.contains_key(&e.dst_id))
                    .collect::<Vec<_>>();

                debug!("Filtered to {:} {:} edges", filtered_edges.len(), link.label);

                for edge in filtered_edges.into_iter() {
                    try!(graph.add_edge(edge))
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
    let rows = try!(connection.query(&*statement, &[]));

    debug!("Loaded {} {} nodes", rows.len(), label);
    let mut nodes = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        let id: String = row.get(0);
        let props: Value = row.get(1);
        let sysan: Value = row.get(2);
        let props = try!(props.as_object().ok_or("Props must be an object"));
        let sysan = try!(sysan.as_object().ok_or("Sysan must be an object"));
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
    let rows = try!(connection.query(&*statement, &[]));

    debug!("Loaded {} {} edges", rows.len(), label);
    let mut edges = Vec::with_capacity(rows.len());

    for row in rows.iter() {
        let src_id: String = row.get(0);
        let dst_id: String = row.get(1);
        let edge = Edge::new(label.clone(), src_id, dst_id);
        edges.push(edge);
    }

    debug!("Instantiated {} {} edges", rows.len(), edge_type.label);
    Ok(edges)
}
