#ifndef STUFF
#define THREADS 512
#define IN_SIZE 1024
#endif

constexpr int threads = THREADS;
constexpr int in_size = IN_SIZE;

extern "C" __global__ void kernel(
    const float* weights,
    const float* biases,
    const float* input,
    const int* moves,
    float* output
) {
    extern __shared__ float sdata[]; 

    const int batch_size = gridDim.y;
    const int loc_in_batch = blockIdx.y;
    const int loc_in_moves = blockIdx.x;
    const int tid = threadIdx.x;
    const int locmb = loc_in_batch * 64 + loc_in_moves;
    const int move = moves[locmb];

    const float4* tW = reinterpret_cast<const float4*>(weights + in_size * move);
    const float4* tI = reinterpret_cast<const float4*>(input + in_size * loc_in_batch);

    if (move != -1)
    {
        float local = 0.0F;

        #pragma unroll
        for (int idx = tid; idx < in_size / 4; idx += threads)
        {
            const float4 tw = tW[idx];
            const float4 ti = tI[idx];
            local += tw.x * ti.x + tw.y * ti.y + tw.z * ti.z + tw.w * ti.w;
        }

        sdata[tid] = local;
        __syncthreads();

        if constexpr (threads >= 1024) { if (tid < 512) sdata[tid] += sdata[tid + 512]; __syncthreads(); }
        if constexpr (threads >= 512) { if (tid < 256) sdata[tid] += sdata[tid + 256]; __syncthreads(); }
        if constexpr (threads >= 256) { if (tid < 128) sdata[tid] += sdata[tid + 128]; __syncthreads(); }
        if constexpr (threads >= 128) { if (tid < 64) sdata[tid] += sdata[tid + 64]; __syncthreads(); }

        if (tid < 32)
        {
            float partial = sdata[tid];
            if constexpr (threads >= 64) { partial += sdata[tid + 32]; }
            partial += __shfl_down_sync(0xffffffff, partial, 16);
            partial += __shfl_down_sync(0xffffffff, partial, 8);
            partial += __shfl_down_sync(0xffffffff, partial, 4);
            partial += __shfl_down_sync(0xffffffff, partial, 2);
            partial += __shfl_down_sync(0xffffffff, partial, 1);

            if (tid == 0)
            {
                output[locmb] = partial + biases[move];
            }
        }
    }
    else if (tid == 0)
    {
        output[locmb] = -10000.0F;
    }
}
