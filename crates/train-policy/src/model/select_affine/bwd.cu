#ifndef STUFF
#define IN_SIZE 1024
#endif

constexpr int in_size = IN_SIZE;

extern "C" __global__ void kernel(
    const float* weights,
    const float* input,
    const int* moves,
    const float* output_grad,
    float* input_grad,
    float* weights_grad,
    float* biases_grad
) {
    extern __shared__ float sdata[];

    const int batch_size = gridDim.y;
    const int loc_in_batch = blockIdx.y;
    const int loc_in_moves = blockIdx.x;
    const int tid = threadIdx.x;
    const int locmb = loc_in_batch * 64 + loc_in_moves;
    const int move = moves[locmb];
    
    if (move != -1)
    {
        const float grd = output_grad[locmb];

        const float4* tW = reinterpret_cast<const float4*>(weights + in_size * move);
        const float4* tI = reinterpret_cast<const float4*>(input + in_size * loc_in_batch);

        if (tid == 0) atomicAdd(biases_grad + move, grd);

        for (int idx = tid; idx < in_size / 4; idx += blockDim.x)
        {
            const int section = 4 * blockDim.x * (idx / blockDim.x) + tid;
            const float4 ti = tI[idx];
            const float4 tw = tW[idx];

            sdata[4 * tid    ] = ti.x;
            sdata[4 * tid + 1] = ti.y;
            sdata[4 * tid + 2] = ti.z;
            sdata[4 * tid + 3] = ti.w;
            __syncthreads();

            float* tWg = weights_grad + in_size * move + section;
            atomicAdd(tWg                 , grd * sdata[tid                 ]);
            atomicAdd(tWg + blockDim.x    , grd * sdata[tid + blockDim.x    ]);
            atomicAdd(tWg + blockDim.x * 2, grd * sdata[tid + blockDim.x * 2]);
            atomicAdd(tWg + blockDim.x * 3, grd * sdata[tid + blockDim.x * 3]);
            __syncthreads();

            sdata[4 * tid    ] = tw.x;
            sdata[4 * tid + 1] = tw.y;
            sdata[4 * tid + 2] = tw.z;
            sdata[4 * tid + 3] = tw.w;
            __syncthreads();

            float* tIg = input_grad + in_size * loc_in_batch + section;
            atomicAdd(tIg                 , grd * sdata[tid                 ]);
            atomicAdd(tIg + blockDim.x    , grd * sdata[tid + blockDim.x    ]);
            atomicAdd(tIg + blockDim.x * 2, grd * sdata[tid + blockDim.x * 2]);
            atomicAdd(tIg + blockDim.x * 3, grd * sdata[tid + blockDim.x * 3]);
            __syncthreads();
        }
    }
}
