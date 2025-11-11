#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub mod blind_arb {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        contract BlindArb {
            function executeArb__WETH_token0(
                address v2Pair,
                address v3Pair,
                uint256 amountIn,
                uint256 percentageToPayToCoinbase
            ) external;

            function executeArb__WETH_token1(
                address v2Pair,
                address v3Pair,
                uint256 amountIn,
                uint256 percentageToPayToCoinbase
            ) external;

            function uniswapV3SwapCallback(
                int256 amount0Delta,
                int256 amount1Delta,
                bytes data
            ) external;

            function withdrawETHToOwner() external;

            function withdrawWETHToOwner() external;

            function owner() external view returns (address);

            function transferOwnership(address newOwner) external;
        }
    }
}

pub mod iweth {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        interface IWETH {
            function deposit() external payable;

            function withdraw(uint256 amount) external;

            function balanceOf(address account) external view returns (uint256);

            function transfer(address recipient, uint256 amount) external returns (bool);
        }
    }
}

pub mod i_uniswap_v2_pair {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        interface IUniswapV2Pair {
            function getReserves()
                external
                view
                returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);

            function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes data) external;
        }
    }
}

pub mod i_uniswap_v3_pool {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        interface IUniswapV3Pool {
            function swap(
                address recipient,
                bool zeroForOne,
                int256 amountSpecified,
                uint160 sqrtPriceLimitX96,
                bytes data
            ) external returns (int256 amount0, int256 amount1);
        }
    }
}

pub mod i_uniswap_v3_swap_callback {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        interface IUniswapV3SwapCallback {
            function uniswapV3SwapCallback(
                int256 amount0Delta,
                int256 amount1Delta,
                bytes data
            ) external;
        }
    }
}
