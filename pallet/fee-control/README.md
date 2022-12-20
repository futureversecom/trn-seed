# Fee Control Pallet

This pallet aims to provide a centralized control panel control the EVM + Substrate extrinsic fees of the chain. It accomplishes this through setting the Base Fee for the EVM, and the multiplication factor for the weight-to-fee calculation. The pallet aims to alter the fees in a safe way by altering the fees per gas/weight unit as opposed to a flat change.