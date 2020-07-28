use petgraph::{graph::NodeIndex, Direction, Graph};
use std::fmt::Write;

const RECURSION_DEPTH_LIMIT: usize = 10;

fn htmlescape(s: &str) -> askama_escape::Escaped<askama_escape::Html> {
    askama_escape::escape(s, askama_escape::Html)
}

fn generate_html(articles: &[String]) -> String {
    let mut graph = petgraph::Graph::new();
    let root = graph.add_node(String::new());
    for article in articles {
        let mut cur = root;
        for component in article.split('/') {
            if let Some(node) = graph
                .neighbors_directed(cur, Direction::Outgoing)
                .find(|node| graph.node_weight(*node).unwrap() == component)
            {
                cur = node;
            } else {
                let node = graph.add_node(component.to_string());
                graph.add_edge(cur, node, ());
                cur = node;
            }
        }
    }

    let mut out = String::new();
    let mut prefix = Vec::with_capacity(RECURSION_DEPTH_LIMIT);
    let mut depth = 0;
    recurse(&mut out, &graph, root, None, &mut prefix, &mut depth);

    out
}

fn recurse<'a>(
    out: &mut String,
    graph: &'a Graph<String, (), petgraph::Directed, u32>,
    node: NodeIndex<u32>,
    node_content: Option<&'a str>,
    prefix: &mut Vec<&'a str>,
    depth: &mut u8,
) {
    if *depth >= RECURSION_DEPTH_LIMIT as u8 {
        return;
    }
    *depth += 1;

    let mut children = graph
        .neighbors_directed(node, Direction::Outgoing)
        .filter_map(|node| graph.node_weight(node).map(|s| (s, node)))
        .collect::<Vec<_>>();

    if let Some(node_content) = node_content {
        out.push_str("<li>");
        let mut link = prefix.join("/");
        write!(link, "/{}", node_content).unwrap();
        write!(
            out,
            "<a href=\"/wiki/{}\">{}</a>",
            htmlescape(&link),
            htmlescape(node_content)
        )
        .unwrap();
    }
    if !children.is_empty() {
        children.sort_unstable_by(|a, b| a.0.cmp(b.0));
        if let Some(node_content) = node_content {
            prefix.push(node_content);
        }
        out.push_str("<ul>");
        for (s, idx) in children {
            recurse(out, graph, idx, Some(s), prefix, depth);
        }
        prefix.pop();
        out.push_str("</ul>");
    }

    if let Some(_) = node_content {
        out.push_str("</li>");
    }

    *depth -= 1;
}

// FIXME: cache this because it's really expensive
// maybe put the cache in Index, needed to put head there anyway
pub fn show_root(
    ctx: crate::context::Context,
    account: Option<crate::user_storage::UserAccount>,
) -> impl warp::Reply {
    render!(crate::templates::Root {
        content: generate_html(&ctx.index.titles()),
        wiki: ctx.wiki(&account)
    })
}
