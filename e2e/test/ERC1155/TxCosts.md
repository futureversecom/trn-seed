## Generated tx costs(Gas) for ERC1155 Precompiles

| Function Call         | Contract gas | Precompile gas | (Extrinsic fee/Gas price) |
|:----------------------|:------------:|:--------------:|:-------------------------:|
| uri                   |    27560     |     22400      |             0             |
| balanceOf             |    25957     |     22433      |             0             |
| balanceOfBatch        |    32585     |     24106      |             0             |
| setApprovalForAll     |    47025     |     27563      |             0             |
| isApprovedForAll      |    26076     |     23184      |             0             |
| safeTransferFrom      |    59163     |     32322      |           23335           |
| safeBatchTransferFrom |    50205     |     32716      |           26668           |
| mint                  |    33152     |     32020      |           23502           |
| mintBatch             |    42210     |     32407      |           26835           |
| burn                  |    32581     |     27731      |           20002           |
| burnBatch             |    38043     |     32004      |           23335           |


## Generated tx costs(fees) for ERC1155 Precompiles

| Function Call         | Contract cost (Drops) | Precompile cost (Drops) | Extrinsic cost (Drops) |
|:----------------------|:---------------------:|:-----------------------:|:----------------------:|
| safeTransferFrom      |        880468         |         467386          |         350034         |
| safeBatchTransferFrom |        728112         |         479987          |         400034         |
| mint                  |        493954         |         457695          |         352538         |
| mintBatch             |        589858         |         470117          |         402538         |
| burn                  |        475652         |         412796          |         300034         |
| burnBatch             |        569231         |         457185          |         350034         |


## Generated tx estimates vs gas used for ERC1155 Precompiles

| Function Call         | Contract estimate | Contract actual | Diff | Precompile estimate | Precompile actual | Diff |
|:----------------------|:-----------------:|:---------------:|:----:|:-------------------:|:-----------------:|:----:|
| safeTransferFrom      |       59163       |      58692      | 471  |        32322        |       31156       | 1166 |
| safeBatchTransferFrom |       50205       |      48536      | 1669 |        32716        |       31996       | 720  |
| mint                  |       33152       |      32927      | 225  |        32020        |       30510       | 1510 |
| mintBatch             |       42210       |      39320      | 2890 |        32407        |       31338       | 1069 |
| burn                  |       32581       |      31707      | 874  |        27731        |       27517       | 214  |
| burnBatch             |       38043       |      37945      |  98  |        32004        |       30476       | 1528 |
