## Generated tx costs(Gas) for ERC1155 Precompiles

| Function Call         | Contract gas | Precompile gas | (Extrinsic fee/Gas price) |
|:----------------------|:------------:|:--------------:|:-------------------------:|
| uri                   |    27560     |     22400      |             0             |
| balanceOf             |    25957     |     22433      |             0             |
| balanceOfBatch        |    32585     |     24106      |             0             |
| setApprovalForAll     |    47025     |     27563      |             0             |
| isApprovedForAll      |    26076     |     23184      |             0             |
| safeTransferFrom      |    59163     |     32322      |           25180           |
| safeBatchTransferFrom |    50205     |     32716      |           28514           |
| mint                  |    33152     |     32020      |           25549           |
| mintBatch             |    42210     |     32407      |           28883           |
| burn                  |    32581     |     27731      |           21859           |
| burnBatch             |    38043     |     32004      |           25192           |


## Generated tx costs(fees) for ERC1155 Precompiles

| Function Call         | Contract cost (Drops) | Precompile cost (Drops) | Extrinsic cost (Drops) |
|:----------------------|:---------------------:|:-----------------------:|:----------------------:|
| safeTransferFrom      |        440278         |         233716          |         188856         |
| safeBatchTransferFrom |        364092         |         240017          |         213856         |
| mint                  |        247001         |         228870          |         191623         |
| mintBatch             |        294958         |         235082          |         216623         |
| burn                  |        237850         |         206418          |         163945         |
| burnBatch             |        284644         |         228615          |         188945         |


## Generated tx estimates vs gas used for ERC1155 Precompiles

| Function Call         | Contract estimate | Contract actual | Diff | Precompile estimate | Precompile actual | Diff |
|:----------------------|:-----------------:|:---------------:|:----:|:-------------------:|:-----------------:|:----:|
| safeTransferFrom      |       59163       |      58692      | 471  |        32322        |       31156       | 1166 |
| safeBatchTransferFrom |       50205       |      48536      | 1669 |        32716        |       31996       | 720  |
| mint                  |       33152       |      32927      | 225  |        32020        |       30510       | 1510 |
| mintBatch             |       42210       |      39320      | 2890 |        32407        |       31338       | 1069 |
| burn                  |       32581       |      31707      | 874  |        27731        |       27517       | 214  |
| burnBatch             |       38043       |      37945      |  98  |        32004        |       30476       | 1528 |
