from substrateinterface import SubstrateInterface
import json
import sys

output_file = sys.argv[1]
url = "ws://127.0.0.1:9944" if len(sys.argv) <= 2 else sys.argv[2]

substrate = SubstrateInterface(url=url)
module_list = substrate.get_metadata_modules()

with open(output_file, 'w') as outfile:
    json.dump(module_list, outfile, indent=2)
