// Result removed
use crate::engine::Pipeline;

pub struct GraphExporter;

impl GraphExporter {
    pub fn to_dot(pipeline: &Pipeline) -> String {
        let mut dot = format!(
            "anygraph G {{\n  label=\"{}\";\n  node [shape=box, style=filled, color=lightblue];\n",
            pipeline.name
        );

        for (i, task) in pipeline.tasks.iter().enumerate() {
            dot.push_str(&format!("  task_{} [label=\"{}\"];\n", i, task.name()));
            if i > 0 {
                dot.push_str(&format!("  task_{} -> task_{};\n", i - 1, i));
            }
        }

        dot.push_str("}\n");
        dot
    }
}
