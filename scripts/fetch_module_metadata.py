from substrateinterface import SubstrateInterface
import json
import sys

substrate = SubstrateInterface(url="ws://127.0.0.1:9944")
module_list = substrate.get_metadata_modules()

with open(sys.argv[1], 'w') as outfile:
    json.dump(module_list, outfile, indent=2)
