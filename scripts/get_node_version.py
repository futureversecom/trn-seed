from concurrent.futures import ThreadPoolExecutor
from substrateinterface import SubstrateInterface
import subprocess
import json
import threading
import xxhash
import os
import argparse
import yaml

FORK_SPEC, RAW_STORAGE = "./output/fork.json", "./output/raw_storage.json"


def fetch_storage_keys_task(hash, keys, prefixes, lock, url):
    substrate = SubstrateInterface(url=url)
    rpc_result = []

    while True:
        lock.acquire()
        if rpc_result:
            keys += rpc_result

        if not prefixes:
            lock.release()
            return

        prefix = prefixes.pop()
        lock.release()

        rpc_result = substrate.rpc_request(method='state_getKeys', params={
            "prefix": prefix, "at": hash})['result']


def fetch_storage_keys(hash, url):
    keys = []
    prefixes = [f'0x{hex(i)[2:].zfill(2)}' for i in range(256)]
    lock = threading.Lock()

    with ThreadPoolExecutor(max_workers=20) as executor:
        for _ in range(20):
            executor.submit(fetch_storage_keys_task,
                            hash, keys, prefixes, lock, url)

    return keys


def fetch_storage_values_task(hash, lock, keys, key_values, url):
    substrate = SubstrateInterface(url=url)

    while True:
        lock.acquire()
        keys_to_fetch = [keys.pop() for _ in range(min(2000, len(keys)))]
        if not keys_to_fetch:
            lock.release()
            return
        lock.release()

        key_values_result = substrate.rpc_request(method='state_queryStorageAt', params={
            "keys": keys_to_fetch, "at": hash})['result'][0]['changes']

        for i, kv in enumerate(key_values_result):
            if kv[1] is None:
                kv[1] = substrate.rpc_request(method='state_getStorage', params={
                    "key": kv[0], "at": hash})['result']

        lock.acquire()
        key_values += key_values_result
        lock.release()


def fetch_storage_values(hash, keys, url):
    key_values = []
    lock = threading.Lock()

    with ThreadPoolExecutor(max_workers=10) as executor:
        for _ in range(10):
            executor.submit(fetch_storage_values_task,
                            hash, lock, keys, key_values, url)

    with open(RAW_STORAGE, 'w') as outfile:
        json.dump(key_values, outfile, indent=2)

    return key_values


def xxh6464(x):
    o1 = bytearray(xxhash.xxh64(x, seed=0).digest())
    o1.reverse()
    o2 = bytearray(xxhash.xxh64(x, seed=1).digest())
    o2.reverse()
    return "0x{}{}".format(o1.hex(), o2.hex())


def list_of_prefixes_to_migrate(substrate):
    # Importing these modules will cause the chain to not work correctly
    skip_modules = ['System', 'Session', 'Babe', 'Grandpa',
                    'GrandpaFinality', 'FinalityTracker', 'Authorship']

    # We definitely want to keep System.Account data and the Runtime :)
    enabled_prefixes = [
        '0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9', '0x3a636f6465']

    module_list = substrate.get_metadata_modules()

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


def populate_dev_chain(substrate, forked_storage, chain_name):
    # Read base chain specification. This will be populated with new storage.
    with open(FORK_SPEC) as in_file:
        base_chain = json.load(in_file)
        base_storage = base_chain['genesis']['raw']['top']

    base_chain['name'] = chain_name + " Fork"

    allowed_prefixes: list[str] = list_of_prefixes_to_migrate(substrate)

    # Dev Sudo Key
    sudo_key_prefix = "0x5c0d1176a568c1f92944340dbfed9e9c530ebca703c85910e7164cb7d1c9e47b"
    sudo_key = base_storage[sudo_key_prefix]

    # Migrate storage from Copied to Base storage
    key: str
    for (key, value) in forked_storage:
        if allowed_to_migrate(key, allowed_prefixes):
            base_storage[key] = value

    # Let's change the sudo key to be Alith :)
    base_storage[sudo_key_prefix] = sudo_key

    # Delete System.LastRuntimeUpgrade to ensure that the on_runtime_upgrade event is triggered
    base_storage.pop(
        '0x26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8')

    # To prevent the validator set from changing mid-test, set Staking.ForceEra to ForceNone ('0x02')
    base_storage['0x5f3e4907f716ac89b6347d15ececedcaf7dad0317324aecae8744b87fc95f2f3'] = '0x02'

    # Write the updated base chain specification to a file
    with open(FORK_SPEC, 'w') as outfile:
        json.dump(base_chain, outfile, indent=2)


def read_configuration_file():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', '-c')
    args = parser.parse_args()

    f = open(args.config, "r")
    configuration = yaml.safe_load(f)

    return configuration


def connect_to_remote_chain(url) -> SubstrateInterface:
    substrate = SubstrateInterface(url=url)
    substrate.init_runtime()
    chain_name = substrate.rpc_request('system_chain', None)['result']

    return (substrate, chain_name)


def determine_node_version(substrate: SubstrateInterface, hash: str) -> str:
    client_version = substrate.rpc_request('system_version', None)[
        'result'].split('.')[0]
    runtime_version = substrate.rpc_request(method='state_getRuntimeVersion', params=[hash])[
        'result']['specVersion']

    print(f'version is v{client_version}.{runtime_version}.0')

    version = f'v{client_version}.{runtime_version}.0'
    all_tags = subprocess.run(
        'git tag', shell=True, text=True, check=True, capture_output=True).stdout
    all_tags = all_tags.splitlines()

    print(f'tags {all_tags}')

    # If the version is not found then we need to do some magic
    #if version not in all_tags:
    #    version = ''
    #    for tag in all_tags:
    #        sub_strings = tag.split('.')
    #        if (sub_strings[1] == f'{runtime_version}'):
    #            version = tag

    if version == '':
        print("Wasn't able to find the correct tag")
        exit(1)

    return version


def maybe_do_tag_switch(tag_switch, node_version):
    if not tag_switch:
        return None

    current_branch = subprocess.run(
        'git branch --show-current', shell=True, text=True, check=True, capture_output=True)

    use_stash = False
    not_committed_changed = subprocess.run(
        'git status --porcelain', shell=True, text=True, check=True, capture_output=True).stdout
    if len(not_committed_changed) > 0:
        use_stash = True
        subprocess.run(
            'git stash', shell=True, text=True, check=True, capture_output=True)

    cmd = f'git checkout {node_version}'
    subprocess.run(cmd, shell=True, text=True, check=True)

    return (current_branch.stdout, use_stash)


def main():
    configuration = read_configuration_file()
    url, tag_switch = configuration['endpoint'], configuration['tag_switch']

    (substrate, chain_name) = connect_to_remote_chain(url)
    hash = configuration.get('at') if configuration.get(
        'at') is not None else substrate.block_hash
    print(
        f"Connected to remote chain: Url: {url}, Chain Name: {chain_name}, Hash: {hash}")

    node_version = determine_node_version(substrate, hash)
    print(f"Node version: {node_version}")

    if not os.path.exists('./output'):
        os.mkdir('./output')
        print("Created output directory: ./output")

    tag_switch = maybe_do_tag_switch(tag_switch, node_version)

    print("Success :)")


if __name__ == "__main__":
    main()
