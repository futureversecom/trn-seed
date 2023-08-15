## Generated tx costs(Gas) for ERC1155 Precompiles

| Function Call         | Contract gas | Precompile gas | (Extrinsic fee/Gas price) |
|:----------------------|:------------:|:--------------:|:-------------------------:|
| uri                   |    27560     |     22400      |             0             |
| balanceOf             |    25957     |     22433      |             0             |
| balanceOfBatch        |    32585     |     24106      |             0             |
| setApprovalForAll     |    47025     |     26460      |             0             |
| isApprovedForAll      |    26076     |     23184      |             0             |
| safeTransferFrom      |    56523     |     31766      |           23335           |
| safeBatchTransferFrom |    44583     |     32160      |           26668           |
| mint                  |    31840     |     29278      |           23502           |
| mintBatch             |    36913     |     31817      |           26835           |
| burn                  |    28958     |     26435      |           20001           |
| burnBatch             |    35511     |     29291      |           23335           |


## Generated tx costs(fees) for ERC1155 Precompiles

| Function Call         | Contract cost (Drops) | Precompile cost (Drops) | Extrinsic cost (Drops) |
|:----------------------|:---------------------:|:-----------------------:|:----------------------:|
| safeTransferFrom      |        838463         |         449594          |         350029         |
| safeBatchTransferFrom |        644104         |         462196          |         400029         |
| mint                  |        451950         |         438808          |         352533         |
| mintBatch             |        547854         |         451230          |         402533         |
| burn                  |        433648         |         394944          |         300029         |
| burnBatch             |        527227         |         439333          |         350029         |


## Generated tx estimates vs gas used for ERC1155 Precompiles

| Function Call         | Contract estimate | Contract actual | Diff | Precompile estimate | Precompile actual | Diff |
|:----------------------|:-----------------:|:---------------:|:----:|:-------------------:|:-----------------:|:----:|
| safeTransferFrom      |       56523       |      55892      | 631  |        31766        |       29970       | 1796 |
| safeBatchTransferFrom |       44583       |      42936      | 1647 |        32160        |       30810       | 1350 |
| mint                  |       31840       |      30127      | 1713 |        29278        |       29251       |  27  |
| mintBatch             |       36913       |      36520      | 393  |        31817        |       30079       | 1738 |
| burn                  |       28958       |      28907      |  51  |        26435        |       26327       | 108  |
| burnBatch             |       35511       |      35145      | 366  |        29291        |       29286       |  5   |
