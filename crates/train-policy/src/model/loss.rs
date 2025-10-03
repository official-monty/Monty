use std::num::NonZeroUsize;

use acyclib::{
    dag::NodeId,
    device::{
        function::{self, DeviceFunction},
        tensor::Shape,
    },
    graph::{
        builder::GraphBuilderNode,
        ir::{
            node::AnnotatedNode,
            operation::{
                binary::SoftmaxCrossEntropy, GraphIROperationBase, GraphIROperationCompilable,
            },
            GraphIR, GraphIRError,
        },
        Graph, GraphNodeIdTy,
    },
};
use bullet_cuda_backend::{
    kernel::{Expr, Kernel, KernelArgs, KernelInput},
    CudaDevice, CudaMarker,
};

use super::MAX_MOVES;

#[derive(Clone, Debug)]
pub struct OptimisedSoftmaxCrossEntropy(SoftmaxCrossEntropy);

impl OptimisedSoftmaxCrossEntropy {
    pub fn new<'a>(
        logits: GraphBuilderNode<'a, CudaMarker>,
        targets: GraphBuilderNode<'a, CudaMarker>,
    ) -> Self {
        Self(SoftmaxCrossEntropy {
            logits: logits.annotated_node(),
            targets: targets.annotated_node(),
        })
    }
}

impl GraphIROperationBase<CudaMarker> for OptimisedSoftmaxCrossEntropy {
    fn nodes(&self) -> Vec<AnnotatedNode> {
        GraphIROperationBase::<CudaMarker>::nodes(&self.0)
    }

    fn output_shape(&self, ir: &GraphIR<CudaMarker>) -> Result<Shape, GraphIRError> {
        assert_eq!(self.0.logits.shape, Shape::new(MAX_MOVES, 1));
        self.0.output_shape(ir)
    }

    fn ancillary_buffers(
        &self,
        ir: &GraphIR<CudaMarker>,
    ) -> Result<Vec<(Shape, Option<NonZeroUsize>, bool)>, GraphIRError> {
        self.0.ancillary_buffers(ir)
    }
}

impl GraphIROperationCompilable<CudaMarker> for OptimisedSoftmaxCrossEntropy {
    fn forward_pass(
        &self,
        graph: &Graph<CudaDevice>,
        output_node: NodeId,
    ) -> DeviceFunction<CudaDevice> {
        let logits = graph.get_ref(self.0.logits.idx, GraphNodeIdTy::Values);
        let targets = graph.get_ref(self.0.targets.idx, GraphNodeIdTy::Values);
        let smax = graph.get_ref(output_node, GraphNodeIdTy::Ancillary(0));
        let output = graph.get_ref(output_node, GraphNodeIdTy::Values);

        let mut func = DeviceFunction::default();

        func.push(function::MaybeUpdateBatchSize {
            input: logits.clone(),
            output: smax.clone(),
        });
        func.push(function::MaybeUpdateBatchSize {
            input: logits.clone(),
            output: output.clone(),
        });

        let threads = 512;
        let entries_per_block = threads / 32;
        let batch_size = Expr::Var;
        let blocks = (batch_size.clone() + entries_per_block - 1) / entries_per_block;
        let grid_dim = [blocks, Expr::Const(1), Expr::Const(1)];
        let block_dim = [threads, 1, 1].map(Expr::Const);
        let shared_mem_bytes = Expr::Const(0);

        let layout = None;
        let batched = logits.batch_size().is_some();
        let shape = Shape::new(MAX_MOVES, 1);

        let inputs = vec![
            KernelInput::Size(batch_size),
            KernelInput::Slice {
                slice: logits,
                layout,
                mutable: false,
                batched,
                shape,
            },
            KernelInput::Slice {
                slice: smax.clone(),
                layout,
                mutable: true,
                batched,
                shape,
            },
        ];

        let args = KernelArgs {
            inputs,
            grid_dim,
            block_dim,
            shared_mem_bytes,
        };

        let code = include_str!("loss/softmax.cu")
            .lines()
            .skip(5)
            .map(|x| format!("{x}\n"))
            .collect::<String>()
            .replace("THREADS", &threads.to_string())
            .replace("SIZE", &MAX_MOVES.to_string());

        let kernel = unsafe { Kernel::new("Softmax".to_string(), code, args) };

        func.push(kernel.unwrap());

        func.push(function::CrossEntropy {
            a: smax,
            b: targets,
            output,
        });

        func
    }

    fn backward_pass(
        &self,
        graph: &Graph<CudaDevice>,
        output_node: NodeId,
    ) -> DeviceFunction<CudaDevice> {
        GraphIROperationCompilable::<CudaMarker>::backward_pass(&self.0, graph, output_node)
    }
}
