use acyclib::{
    dag::NodeId,
    device::{
        function::{DeviceFunction, MaybeUpdateBatchSize},
        tensor::Shape,
    },
    graph::{
        builder::{Affine, GraphBuilderNode},
        ir::{
            node::AnnotatedNode,
            operation::{util, GraphIROperationBase, GraphIROperationCompilable},
            BackendMarker, GraphIR, GraphIRError,
        },
        Graph, GraphNodeIdTy,
    },
};
use bullet_cuda_backend::{
    kernel::{Expr, Kernel, KernelArgs, KernelInput},
    CudaDevice, CudaMarker,
};

use monty::networks::policy::outputs::NUM_MOVES_INDICES;

use super::MAX_MOVES;

#[derive(Debug)]
pub struct SelectAffine {
    weights: AnnotatedNode,
    biases: AnnotatedNode,
    input: AnnotatedNode,
    indices: AnnotatedNode,
}

impl SelectAffine {
    pub fn new<'a>(
        affine: Affine<'a, CudaMarker>,
        input: GraphBuilderNode<'a, CudaMarker>,
        indices: GraphBuilderNode<'a, CudaMarker>,
    ) -> Self {
        Self {
            weights: affine
                .weights
                .reshape(affine.weights.annotated_node().shape.transpose())
                .annotated_node(),
            biases: affine.bias.annotated_node(),
            input: input.annotated_node(),
            indices: indices.annotated_node(),
        }
    }
}

impl<B: BackendMarker> GraphIROperationBase<B> for SelectAffine {
    fn nodes(&self) -> Vec<AnnotatedNode> {
        vec![self.indices, self.input, self.weights, self.biases]
    }

    fn output_shape(&self, ir: &GraphIR<B>) -> Result<Shape, GraphIRError> {
        assert_eq!(
            self.weights.shape,
            Shape::new(self.input.shape.rows(), NUM_MOVES_INDICES)
        );
        assert_eq!(self.biases.shape, Shape::new(NUM_MOVES_INDICES, 1));

        util::check_same_batching(ir, &[&self.indices, &self.input])?;
        util::check_dense_eq(ir, &self.input, true)?;
        util::check_dense_eq(ir, &self.indices, false)?;
        util::check_dense_eq(ir, &self.weights, true)?;
        util::check_dense_eq(ir, &self.biases, true)?;
        util::check_not_batched(ir, &self.weights)?;
        util::check_not_batched(ir, &self.biases)?;

        Ok(Shape::new(MAX_MOVES, 1))
    }
}

impl GraphIROperationCompilable<CudaMarker> for SelectAffine {
    fn forward_pass(
        &self,
        graph: &Graph<CudaDevice>,
        output_node: NodeId,
    ) -> DeviceFunction<CudaDevice> {
        let input = graph.get_ref(self.input.idx, GraphNodeIdTy::Values);
        let indices = graph.get_ref(self.indices.idx, GraphNodeIdTy::Values);
        let weights = graph.get_ref(self.weights.idx, GraphNodeIdTy::Values);
        let biases = graph.get_ref(self.biases.idx, GraphNodeIdTy::Values);
        let output = graph.get_ref(output_node, GraphNodeIdTy::Values);

        let mut func = DeviceFunction::default();

        func.push(MaybeUpdateBatchSize {
            input: input.clone(),
            output: output.clone(),
        });

        let single_size = input.single_size();
        let batch_size = Expr::Var;
        let threads = (single_size / 4).min(512) as i32;
        let grid_dim = [Expr::Const(64), batch_size, Expr::Const(1)];
        let block_dim = [threads, 1, 1].map(Expr::Const);
        let shared_mem_bytes = Expr::Const(4 * threads);

        assert!(
            (threads as u32).is_power_of_two(),
            "hl size must be a power of 2"
        );
        assert_eq!(MAX_MOVES, 64);

        let layout = None;
        let mutable = false;

        let inputs = vec![
            KernelInput::Slice {
                slice: weights,
                layout,
                mutable,
                batched: false,
                shape: self.weights.shape,
            },
            KernelInput::Slice {
                slice: biases,
                layout,
                mutable,
                batched: false,
                shape: self.biases.shape,
            },
            KernelInput::Slice {
                slice: input,
                layout,
                mutable,
                batched: true,
                shape: self.input.shape,
            },
            KernelInput::Slice {
                slice: indices,
                layout: Some(64),
                mutable,
                batched: true,
                shape: self.indices.shape,
            },
            KernelInput::Slice {
                slice: output,
                layout,
                mutable: true,
                batched: true,
                shape: Shape::new(MAX_MOVES, 1),
            },
        ];

        let args = KernelArgs {
            inputs,
            block_dim,
            grid_dim,
            shared_mem_bytes,
        };

        let code = include_str!("select_affine/fwd.cu")
            .lines()
            .skip(5)
            .map(|x| format!("{x}\n"))
            .collect::<String>()
            .replace("THREADS", &threads.to_string())
            .replace("IN_SIZE", &single_size.to_string());

        let kernel = unsafe { Kernel::new("SelectAffineFwd".to_string(), code, args) };

        func.push(kernel.unwrap());

        func
    }

    fn backward_pass(
        &self,
        graph: &Graph<CudaDevice>,
        output_node: NodeId,
    ) -> DeviceFunction<CudaDevice> {
        let input = graph.get_ref(self.input.idx, GraphNodeIdTy::Values);
        let indices = graph.get_ref(self.indices.idx, GraphNodeIdTy::Values);
        let weights = graph.get_ref(self.weights.idx, GraphNodeIdTy::Values);
        let input_grad = graph.get_ref(self.input.idx, GraphNodeIdTy::Gradients);
        let weights_grad = graph.get_ref(self.weights.idx, GraphNodeIdTy::Gradients);
        let biases_grad = graph.get_ref(self.biases.idx, GraphNodeIdTy::Gradients);
        let output_grad = graph.get_ref(output_node, GraphNodeIdTy::Gradients);

        let mut func = DeviceFunction::default();

        func.push(MaybeUpdateBatchSize {
            input: output_grad.clone(),
            output: input_grad.clone(),
        });

        assert_eq!(MAX_MOVES, 64);

        let single_size = input.single_size();
        let batch_size = Expr::Var;
        let threads = (single_size / 4).min(1024) as i32;
        let grid_dim = [Expr::Const(64), batch_size, Expr::Const(1)];
        let block_dim = [threads, 1, 1].map(Expr::Const);
        let shared_mem_bytes = Expr::Const(16 * threads);

        let layout = None;
        let mutable = false;

        let inputs = vec![
            KernelInput::Slice {
                slice: weights,
                layout,
                mutable,
                batched: false,
                shape: self.weights.shape,
            },
            KernelInput::Slice {
                slice: input,
                layout,
                mutable,
                batched: true,
                shape: self.input.shape,
            },
            KernelInput::Slice {
                slice: indices,
                layout: Some(64),
                mutable,
                batched: true,
                shape: self.indices.shape,
            },
            KernelInput::Slice {
                slice: output_grad,
                layout,
                mutable,
                batched: true,
                shape: Shape::new(MAX_MOVES, 1),
            },
            KernelInput::Slice {
                slice: input_grad,
                layout,
                mutable: true,
                batched: true,
                shape: self.input.shape,
            },
            KernelInput::Slice {
                slice: weights_grad,
                layout,
                mutable: true,
                batched: false,
                shape: self.weights.shape,
            },
            KernelInput::Slice {
                slice: biases_grad,
                layout,
                mutable: true,
                batched: false,
                shape: self.biases.shape,
            },
        ];

        let args = KernelArgs {
            inputs,
            grid_dim,
            block_dim,
            shared_mem_bytes,
        };

        let code = include_str!("select_affine/bwd.cu")
            .lines()
            .skip(4)
            .map(|x| format!("{x}\n"))
            .collect::<String>()
            .replace("IN_SIZE", &single_size.to_string());

        let kernel = unsafe { Kernel::new("SelectAffineBwd".to_string(), code, args) };

        func.push(kernel.unwrap());

        func
    }
}
