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


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', '-c')
    parser.add_argument('--branch', '-b')
    args = parser.parse_args()

    f = open(args.config, "r")
    configuration = yaml.safe_load(f)

    release_candidate = args.branch

    return (configuration, release_candidate)


def connect_to_remote_chain(url) -> SubstrateInterface:
    substrate = SubstrateInterface(url=url)
    substrate.init_runtime()
    chain_name = substrate.rpc_request('system_chain', None)['result']

    return (substrate, chain_name)


def determine_node_version(substrate: SubstrateInterface, hash: str):
    client_version = substrate.rpc_request('system_version', None)[
        'result'].split('.')[0]
    runtime_version = substrate.rpc_request(method='state_getRuntimeVersion', params=[hash])[
        'result']['specVersion']

    version = f'v{client_version}.{runtime_version}.0'
    all_tags = subprocess.run(
        'git tag', shell=True, text=True, check=True, capture_output=True).stdout
    all_tags = all_tags.splitlines()

    # If the version is not found then we need to do some magic
    if version not in all_tags:
        version = ''
        for tag in all_tags:
            sub_strings = tag.split('.')
            if (sub_strings[1] == f'{runtime_version}'):
                version = tag

    if version == '':
        print("Wasn't able to find the correct tag")
        exit(1)

    return version


def build_runtime_upgrade_wasm(release_candidate):
    if not release_candidate:
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

    cmd = f'git checkout {release_candidate}'
    subprocess.run(cmd, shell=True, text=True, check=True)

    # Build the runtime upgrade wasm
    subprocess.run('cargo build --release --package seed-runtime',
                   shell=True, text=True, check=True, capture_output=True)

    # Copy wasm to output
    subprocess.run('cp target/release/wbuild/seed-runtime/*.wasm ./output/',
                   shell=True, text=True, check=True, capture_output=True)

    return (current_branch.stdout, use_stash)


def maybe_do_tag_switch(tag_switch, node_version):
    if not tag_switch:
        return None

    # TODO (remove later) Copy scripts
    subprocess.run('cp scripts/*.py ./output/ && cp dockerimages/fork-state.Dockerfile ./output/',
                   shell=True, text=True, check=True, capture_output=True)

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

    # TODO (remove later) Copy scripts
    subprocess.run('cp ./output/*.py ./scripts/ && cp ./output/fork-state.Dockerfile ./dockerimages/',
                   shell=True, text=True, check=True, capture_output=True)

    return (current_branch.stdout, use_stash)


def main():
    (configuration, release_candidate) = parse_args()
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

    build_runtime = build_runtime_upgrade_wasm(release_candidate)

    tag_switch = maybe_do_tag_switch(tag_switch, node_version)

    print("Success :)")


if __name__ == "__main__":
    main()
