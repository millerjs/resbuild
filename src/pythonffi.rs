use ::errors::EBError;
use ::graph::{connect, CachingOptions, CachedGraph};
use ::datamodel::Datamodel;
use cpython::{PyErr, Python, PyResult, PyDict};

/// Create custom Python Exception
py_exception!(resbuild, RustEBError);

impl EBError {
    /// Convert EBError to PyErr (can't impl Into because we require
    /// the Python context)
    pub fn to_pyerr(self, py: Python) -> PyErr {
        RustEBError::new(py, format!("{:?}", self))
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
        Ok(())
    }
);

macro_rules! pytry {
    ($py:expr, $e:expr) => (match $e {
        Ok(val) => val,
        Err(err) => return Err(EBError::from(err).to_pyerr($py)),
    });
}

py_class!(class RustCachedGraph |py| {
    data graph: CachedGraph;

    def __new__(_cls, host: String, database: String, user: String, password: String)
                -> PyResult<RustCachedGraph>
    {
        let caching_options = &CachingOptions::new();
        let datamodel = Datamodel::new().unwrap();
        let connection = pytry!(py, connect(host, database, user, password));
        let graph = pytry!(py, CachedGraph::from_postgres(
            caching_options, &datamodel, &connection));
        RustCachedGraph::create_instance(py, graph)
    }

    def node_count(&self) -> PyResult<usize> {
        Ok(self.graph(py).nodes.len())
    }
});


py_class!(class RustNode |py| {
    data props: PyDict;

    def __new__(_cls) -> PyResult<RustNode> {
        RustNode::create_instance(py, PyDict::new(py))
    }
});
