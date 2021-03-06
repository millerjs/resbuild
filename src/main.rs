#![allow(unused_variables)]
#![allow(unused_mut)]

#[macro_use]
extern crate log;
extern crate esbuild;
extern crate openssl;
extern crate postgres;
extern crate serde;
extern crate serde_json;
extern crate env_logger;
extern crate threadpool;
extern crate scoped_pool;

use esbuild::common::{case_type_tree, denormalize_tree};
use esbuild::errors::EBResult;
use esbuild::graph::{connect, CachedGraph};
use esbuild::types::{Datamodel, CachingOptions, Options, NodeTree};
use postgres::Connection;
use scoped_pool::Pool;
use std::env;
use std::sync::mpsc::channel;


/// Pull in environment variables to create postgres connection
fn env_psql() -> EBResult<Connection> {
    let host = env::var("PG_HOST").unwrap_or("localhost".to_string());
    let database = env::var("PG_DATABASE").unwrap_or("automated_test".to_string());
    let user = env::var("PG_USER").unwrap_or(env::var("USER").unwrap_or("postgres".to_string()));
    let password = env::var("PG_PASSWORD").unwrap_or("".to_string());

    Ok(try!(connect(host, database, user, password)))
}


fn denormalize(graph: &CachedGraph, options: &Options) ->EBResult<()> {
    // Setup denormalization
    let cases = graph.nodes_labeled("case");
    let case_type_tree = &case_type_tree();
    let n_cases = cases.len();
    let pool = Pool::new(16);

    // Do the denormalization
    let (tx, rx) = channel();
    pool.scoped(|scope| {
        for case in cases {
            let tx = tx.clone();
            scope.execute(move || {
                debug!("Denormalizing {:}", case);
                let case_tree = &NodeTree::construct(graph, case_type_tree, case);
                let doc = denormalize_tree(options, graph, case_tree);
                tx.send(doc).unwrap();
            })
        }
    });

    debug!("Collecting cases");
    let case_docs = rx.iter().take(n_cases).collect::<Vec<_>>();
    for case_doc in case_docs {
        println!("{:?}", case_doc);
    }

    Ok(())
}


/// Build the legacy index
fn build_legacy_index() -> EBResult<()> {
    // Construct datamode from included resources
    let mut datamodel = try!(Datamodel::new());

    // Cache the graph
    let connection = try!(env_psql());
    let caching_options = &CachingOptions::new();
    let graph = &try!(CachedGraph::from_postgres(caching_options, &datamodel, &connection));
    let options = &Options::legacy_defaults(datamodel);

    try!(denormalize(graph, options));

    Ok(())
}


fn main() {
    env_logger::init().unwrap();

    if let Err(error) = Datamodel::new() {
        println!("{:?}", error)
    }

    if let Err(error) = build_legacy_index() {
        println!("{:?}", error)
    }
}
