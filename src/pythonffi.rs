use ::datamodel::Datamodel;
use ::errors::EBError;
use ::graph::{connect, CachingOptions, CachedGraph};
use ::node::Node;
use ::edge::Edge;

use env_logger;
use serde_json::Value;

use cpython::{
    PyErr,
    Python,
    PyResult,
    PyDict,
    PythonObject,
    ToPyObject,
    PyObject,
};

/// Create custom Python Exception
py_exception!(resbuild, RustEBError);


impl EBError {
    /// Convert EBError to PyErr (can't impl Into because we require
    /// the Python context)
    pub fn to_pyerr(self, py: Python) -> PyErr {
        RustEBError::new(py, format!("{:?}", self))
    }
}


/// try!() alternative that coerces Into<EBError> into PyErr
macro_rules! pytry {
    ($py:expr, $e:expr) => ($e.map_err(|e| EBError::from(e).to_pyerr($py))?);
}


/// try!() alternative that coerces Into<EBError> into PyErr
macro_rules! pyobj {
    ($py:expr, $val:expr) => ($val.to_py_object($py).into_object());
}


/// Turn serde_json::Value into PyObject
fn extract_json_scalar(py: Python, val: &Value) -> PyObject {
    match *val {
        Value::Null => py.None(),
        Value::Bool(ref val) => pyobj!(py, val),
        Value::I64(ref val) => pyobj!(py, val),
        Value::U64(ref val) => pyobj!(py, val),
        Value::F64(ref val) => pyobj!(py, val),
        Value::String(ref val) => pyobj!(py, val),
        _ => RustEBError::new(py, format!("Unknown type: {:?}", val)).instance(py),
    }
}


/// Create our extension module
py_module_initializer!(
    resbuild,
    initresbuild,
    PyInit_resbuild, |py, m|
    {
        try!(m.add(py, "__doc__", "GDC Datamodel features in rust."));
        try!(m.add_class::<RustCachedGraph>(py));
        try!(m.add_class::<RustNode>(py));
        Ok(())
    }
);


impl Node {
    fn to_py(&self, py: Python) -> PyResult<RustNode> {
        RustNode::create_instance(py, self.clone())
    }
}

impl Edge {
    fn to_py(&self, py: Python) -> PyResult<RustEdge> {
        RustEdge::create_instance(py, self.clone())
    }
}


/// map a Vec<Node> to python list representation
fn map_rustnode(py: Python, nodes: &Vec<&Node>) -> PyResult<Vec<RustNode>> {
    nodes.iter().map(|n| n.to_py(py)).collect()
}

/// Map a Vec<Edge> to python list representation
fn map_rustedge(py: Python, edges: &Vec<Edge>) -> PyResult<Vec<RustEdge>> {
    edges.iter().map(|e| e.to_py(py)).collect()
}


py_class!(class RustCachedGraph |py| {
    data graph: CachedGraph;

    def __new__(_cls, schemas: Vec<String>,
                host: &str, database: &str, user: &str, password: &str)
                -> PyResult<RustCachedGraph>
    {
        env_logger::init().unwrap();

        let caching_options = &CachingOptions::new();
        let datamodel = pytry!(py, Datamodel::new(&schemas));
        let connection = pytry!(py, connect(host, database, user, password));
        let graph = pytry!(py, CachedGraph::from_postgres(
            caching_options, &datamodel, &connection));

        RustCachedGraph::create_instance(py, graph)
    }

    def node_count(&self) -> PyResult<usize> {
        Ok(self.graph(py).nodes.len())
    }

    def get_node(&self, id: String) -> PyResult<RustNode> {
        pytry!(py, self.graph(py).get_node(&id).map(|n| n.to_py(py))
               .ok_or(format!("Node '{}' not found", id)))
    }

    def neighbors(&self, id: String) -> PyResult<Vec<RustNode>> {
        map_rustnode(py, &self.graph(py).neighbors(&id))
    }

    def neighbors_labeled(&self, id: String, labels: Vec<String>) -> PyResult<Vec<RustNode>> {
        map_rustnode(py, &self.graph(py).neighbors_labeled(&id, &labels))
    }

    def nodes_labeled(&self, labels: Vec<String>) -> PyResult<Vec<RustNode>> {
        map_rustnode(py, &self.graph(py).nodes_labeled(&labels))
    }

    def get_edges(&self, src_id: String, dst_id: String) -> PyResult<Vec<RustEdge>> {
        map_rustedge(py, self.graph(py).get_edges(&src_id, &dst_id).unwrap_or(&vec![]))
    }

    def get_node_ids(&self) -> PyResult<Vec<String>> {
        Ok(self.graph(py).nodes.keys().map(|id| id.clone()).collect())
    }

    def remove_nodes_from(&self, ids: Vec<String>) -> PyResult<usize> {
        Ok(ids.iter().map(|id| self.graph(py).remove_node(id))
           .filter(|n| n.is_some()).count())
    }

});


py_class!(class RustNode |py| {
    data data: Node;

    def __repr__(&self) -> PyResult<String> { Ok(self.data(py).to_string())   }
    def label(&self) -> PyResult<String>    { Ok(self.data(py).label.clone()) }
    def node_id(&self) -> PyResult<String>  { Ok(self.data(py).id.clone())    }

    def props(&self) -> PyResult<PyDict> {
        let dict = PyDict::new(py);
        let node = self.data(py);
        for (key, val) in &node.props {
            dict.set_item(py, key, extract_json_scalar(py, val))?;
        }
        Ok(dict)
    }

    def acl(&self) -> PyResult<Vec<String>> {
        Ok(self.data(py).acl.clone())
    }

    def get_prop(&self, key: String) -> PyResult<PyObject> {
        Ok(self.data(py).props.get(&key)
            .map(|v| extract_json_scalar(py, v))
            .unwrap_or(py.None()))
    }

    def get_sysan(&self, key: String) -> PyResult<PyObject> {
        Ok(self.data(py).sysan.get(&key)
            .map(|v| extract_json_scalar(py, v))
            .unwrap_or(py.None()))
    }

    def sysan(&self) -> PyResult<PyDict> {
        let dict = PyDict::new(py);
        let node = self.data(py);
        for (key, val) in &node.sysan {
            dict.set_item(py, key, extract_json_scalar(py, val))?;
        }
        Ok(dict)
    }
});


py_class!(class RustEdge |py| {
    data data: Edge;

    def __repr__(&self) -> PyResult<String> { Ok(self.data(py).to_string())    }
    def label(&self) -> PyResult<String>    { Ok(self.data(py).label.clone())  }
    def src_id(&self) -> PyResult<String>   { Ok(self.data(py).src_id.clone()) }
    def dst_id(&self) -> PyResult<String>   { Ok(self.data(py).src_id.clone()) }
});
