mod utils;

use js_sys::{JsString, Uint8Array};
use wasm_bindgen::prelude::*;

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use oxigraph::io::GraphFormat;
use oxigraph::model::*;
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use petgraph::dot::Dot;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Graph;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::Write;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, web-rdf-class-viz!");
}

// converts a format string into a GraphFormat enum
pub fn format_from_str(format: &str) -> Result<GraphFormat> {
    match format {
        "turtle" => Ok(GraphFormat::Turtle),
        "ttl" => Ok(GraphFormat::Turtle),
        "ntriples" => Ok(GraphFormat::NTriples),
        "rdfxml" => Ok(GraphFormat::RdfXml),
        "triple" => Ok(GraphFormat::NTriples),
        _ => Err(anyhow!("Invalid format")),
    }
}


// wasm_bindgen struct wrapping a visualizer object and abstracting its methods
// so they can be used in JS
#[wasm_bindgen(js_name = Visualizer)]
pub struct VisualizerWrapper {
    visualizer: Visualizer,
}

#[wasm_bindgen(js_class = Visualizer)]
impl VisualizerWrapper {
    pub fn new() -> VisualizerWrapper {
        utils::set_panic_hook();
        VisualizerWrapper {
            visualizer: (Visualizer::new()).unwrap(),
        }
    }

    #[wasm_bindgen(js_name = addOntology)]
    pub fn add_ontology(&mut self, content: JsString, format: JsString) -> () {
        let format = GraphFormat::Turtle;
        let content_str: String = content.into();
        self.visualizer.add_ontology(content_str.as_bytes(), format).unwrap();
    }

    #[wasm_bindgen(js_name = createDotFile)]
    pub fn create_dot_file(&mut self, content: JsString, format: JsString) -> String {
        let format_str = GraphFormat::Turtle;
        let content_str: String = content.into();

        // Call your visualizer function with the converted arguments
        self.visualizer
            .create_graph(content_str.as_bytes(), format_str)
            .unwrap()
    }
}

static PREFIXES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("brick", "https://brickschema.org/schema/Brick#");
    map.insert("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
    map.insert("owl", "http://www.w3.org/2002/07/owl#");
    map
});

fn rewrite_term(node: &Term) -> String {
    let mut s = node.to_string();
    for (prefix, namespace) in PREFIXES.iter() {
        s = s.replace(namespace, format!("{}_", prefix).as_str());
    }
    let matches: &[_] = &['<', '>', '"'];
    s.trim_matches(matches).to_owned()
}

fn graph_to_dot(graph: &petgraph::Graph<&str, &str>, filename: &str) -> Result<()> {
    let mut file = File::create(filename)?;
    write!(file, "{:?}", Dot::with_config(graph, &[]))?;
    Ok(())
}

type ColorFn = fn(node: &str) -> String;
pub type FilterFn = fn(from: &str, to: &str, edge: &str) -> bool;

// if the build target is WASM, then the store inside should be the oxigraph JSStore
// if the build target is native, then the store inside should be the oxigraph Store

pub struct Visualizer {
    store: Store,
    labels: Vec<String>,
    g: Graph<String, String>,
    nodes: HashMap<String, NodeIndex>,
    filter: Option<FilterFn>,
    class_color_map: Option<HashMap<String, String>>,
    colors: HashMap<String, String>,
}

impl Visualizer {
    pub fn new() -> Result<Self> {
        Ok(Visualizer {
            store: Store::new()?,
            labels: Vec::new(),
            g: Graph::new(),
            nodes: HashMap::new(),
            colors: HashMap::new(),
            class_color_map: None,
            filter: None,
        })
    }

    // add FilterFn to Visualizer
    pub fn add_filter(&mut self, filter: FilterFn) {
        self.filter = Some(filter);
    }

    // add class color map to Visualizer
    pub fn add_class_color_map(&mut self, class_color_map: HashMap<String, String>) {
        self.class_color_map = Some(class_color_map);
    }

    pub fn add_ontology(&mut self, content: impl BufRead, format: GraphFormat) -> Result<()> {
        Ok(self
            .store
            .load_graph(content, format, GraphNameRef::DefaultGraph, None)?)
    }

    pub fn graph_to_d2lang(&self) -> Result<String> {
        let mut w = Vec::new();

        // Write edge labels
        for edge in self.g.edge_references() {
            let source = edge.source();
            let target = edge.target();
            let label = edge.weight();
            writeln!(
                w,
                "{} -> {}: {}",
                self.g.node_weight(source).unwrap(),
                self.g.node_weight(target).unwrap(),
                label
            )?;
        }

        // write colors
        for (node, color) in self.colors.iter() {
            writeln!(w, "{}.style.fill: \"{}\"", node, color)?;
        }

        Ok(String::from_utf8(w)?)
    }

    fn to_color(&self, node: &Term) -> Result<String> {
        if let Some(class_color_map) = &self.class_color_map {
            for (class_name, color) in class_color_map.iter() {
                let q = format!(
                    "PREFIX owl: <http://www.w3.org/2002/07/owl#>
                         PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                         ASK {{
                            {0} (rdfs:subClassOf|owl:equivalentClass)* <{1}>
                         }}",
                    node, class_name
                );
                if let QueryResults::Boolean(is_subclass) = self.store.query(&q)? {
                    if is_subclass {
                        return Ok(color.clone());
                    }
                }
            }
        }
        Ok("#ffffff".to_owned())
    }

    pub fn create_graph(
        &mut self,
        data_graph: impl BufRead,
        format: GraphFormat,
    ) -> Result<String> {
        // load into a graph
        self.store
            .load_graph(data_graph, format, GraphNameRef::DefaultGraph, None)?;

        let q = "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
                 PREFIX owl: <http://www.w3.org/2002/07/owl#>
                 SELECT ?from ?p ?to WHERE {
                     ?x rdf:type ?from .
                     ?x ?p ?y .
                     ?y rdf:type ?to .
                     ?from a owl:Class .
                     ?to a owl:Class .
                 }";

        if let QueryResults::Solutions(solutions) = self.store.query(q)? {
            let mut edges: Vec<(usize, usize, usize)> = Vec::new();
            for row in solutions {
                let row = row?;

                {
                    let from = row.get("from").unwrap().to_string();
                    let to = row.get("to").unwrap().to_string();
                    let p = row.get("p").unwrap().to_string();

                    if let Some(filter) = self.filter {
                        if !(filter)(from.as_str(), to.as_str(), p.as_str()) {
                            continue;
                        }
                    }
                }
                let from_term = row.get("from").unwrap();
                let f = rewrite_term(&from_term);
                if !self.colors.contains_key(&f) {
                    self.colors
                        .insert(f.clone(), self.to_color(&from_term).unwrap().to_owned());
                }
                self.labels.push(f);
                let f_idx = self.labels.len() - 1;

                let to_term = row.get("to").unwrap();
                let t = rewrite_term(&to_term);
                if !self.colors.contains_key(&t) {
                    self.colors
                        .insert(t.clone(), self.to_color(&to_term).unwrap().to_owned());
                }
                self.labels.push(t);
                let t_idx = self.labels.len() - 1;

                let e = rewrite_term(row.get("p").unwrap());
                self.labels.push(e);
                let e_idx = self.labels.len() - 1;
                edges.push((f_idx, t_idx, e_idx));
            }

            // Now that we have collected all the data, update the graph outside the loop
            for (from, to, edge) in edges {
                let from_label = self.labels[from].clone();
                let to_label = self.labels[to].clone();

                let from_idx = {
                    if let Some(&idx) = self.nodes.get(&from_label) {
                        idx
                    } else {
                        let new_node = self.g.add_node(from_label.clone());
                        self.nodes.insert(from_label.clone(), new_node);
                        new_node
                    }
                };
                let to_idx = {
                    if let Some(&idx) = self.nodes.get(&to_label) {
                        idx
                    } else {
                        let new_node = self.g.add_node(to_label.clone());
                        self.nodes.insert(to_label.clone(), new_node);
                        new_node
                    }
                };
                self.g
                    .update_edge(from_idx, to_idx, self.labels[edge].clone());
            }
        }

        let mut w = Vec::new();
        write!(w, "{:?}", Dot::with_config(&self.g, &[]))?;
        Ok(String::from_utf8(w)?)
        //graph_to_dot(&self.g, "output.dot")?;
        //self.graph_to_d2lang()
    }
}
