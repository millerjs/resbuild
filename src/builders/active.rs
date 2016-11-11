use ::builders::common::{Builder, Options};
use ::datamodel::Datamodel;
use ::graph::CachedGraph;


pub struct ActiveBuilder<'a> {
    pub graph: &'a CachedGraph,
    pub options: &'a Options,
}


impl Options {
    pub fn active_defaults(datamodel: Datamodel) -> Options {
        Options {
            datamodel: datamodel,
            case_to_file_paths: Vec::new(),
            file_labels: Vec::new(),
            possible_associated_entites: Vec::new(),
            index_file_extensions: Vec::new(),
        }
    }
}

impl<'a> ActiveBuilder<'a> {
    pub fn new(options: &'a Options, graph: &'a CachedGraph) -> ActiveBuilder<'a> {
        ActiveBuilder { options: options, graph: graph }
    }
}


impl<'b> Builder for ActiveBuilder<'b> {
    fn options<'a>(&'a self) -> &'a Options { &self.options }
    fn graph<'a>(&'a self) -> &'a CachedGraph { &self.graph }
}
