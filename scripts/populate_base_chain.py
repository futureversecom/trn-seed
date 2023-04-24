import json
import xxhash
import sys


def xxh6464(x):
    o1 = bytearray(xxhash.xxh64(x, seed=0).digest())
    o1.reverse()
    o2 = bytearray(xxhash.xxh64(x, seed=1).digest())
    o2.reverse()
    return "0x{}{}".format(o1.hex(), o2.hex())


def list_of_prefixes_to_migrate(module_metadata_path: str):
    # Importing these modules will cause the chain to not work correctly
    skip_modules = ['System', 'Session', 'Babe', 'Grandpa',
                    'GrandpaFinality', 'FinalityTracker', 'Authorship']

    # We definitely want to keep System.Account data and the Runtime :)
    enabled_prefixes = [
        '0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9', '0x3a636f6465']

    f = open(module_metadata_path)
    module_list = json.load(f)

    for module in module_list:
        name = module['name']
        if name not in skip_modules:
            enabled_prefixes.append(xxh6464(name))

    return enabled_prefixes


def allowed_to_migrate(key: str, allow_list):
    for prefix in allow_list:
        if key.startswith(prefix):
            return True
    return False


def main():
    base_chain_path = sys.argv[1]
    support_chain_path = sys.argv[2]
    module_metadata_path = sys.argv[3]

    # Read base chain specification. This will be populated with new storage.
    with open(base_chain_path) as in_file:
        base_chain = json.load(in_file)
        base_storage = base_chain['genesis']['raw']['top']

    # Read copied chain specification. This will be used to migrate storage to the base chain.
    with open(support_chain_path) as in_file:
        support_chain = json.load(in_file)
        support_storage = support_chain['genesis']['raw']['top']

    # Generate a list of allowed storage prefixes to be migrated
    allowed_prefixes: list[str] = list_of_prefixes_to_migrate(
        module_metadata_path)

    # Dev Sudo Key
    sudo_key_prefix = "0x5c0d1176a568c1f92944340dbfed9e9c530ebca703c85910e7164cb7d1c9e47b"
    sudo_key = base_storage[sudo_key_prefix]

    # Migrate storage from Copied to Base storage
    key: str
    for key in support_storage:
        if allowed_to_migrate(key, allowed_prefixes):
            base_storage[key] = support_storage[key]

    # Let's change the sudo key to be Alith :)
    base_storage[sudo_key_prefix] = sudo_key

    # Delete System.LastRuntimeUpgrade to ensure that the on_runtime_upgrade event is triggered
    base_storage.pop(
        '0x26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8')

    # To prevent the validator set from changing mid-test, set Staking.ForceEra to ForceNone ('0x02')
    base_storage['0x5f3e4907f716ac89b6347d15ececedcaf7dad0317324aecae8744b87fc95f2f3'] = '0x02'

    # Write the updated base chain specification to a file
    with open(base_chain_path, 'w') as outfile:
        json.dump(base_chain, outfile, indent=2)


if __name__ == "__main__":
    main()
