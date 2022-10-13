# NFT Peg Pallet

This pallet is for:
- Processing incoming bridged messages involving NFTs(ERC721 and ERC1155)
- Mapping bridged NFTs to Root NFTs

### ERC721/1155
As of now, the hope is that this pallet will be able to handle both ERC721 and 1155 tokens, while at the time of writing, only ERC721 tokens have been considered in the implementation, while planning around 1155 is ongoing.

# TODO:
- Storage migrations for Chain origin information, + SerialNumber