# Validator set pallet
This pallet provides functionality required for maintaining the Validator set for the chain.  
Main functionalities
 - Implements the session hooks, receive and maintain active validator set
 - Maintain subset of validators that are required for other pallets. e.g.- XRPL validators for XRPL Bridge
 - Support session/era changes and force era