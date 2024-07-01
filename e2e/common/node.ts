// Adapted  from webb-tools - https://github.com/webb-tools/dkg-substrate/blob/0d86b54f57a38881ef0e555ec757b5324e5c8ca7/dkg-test-suite/tests/utils/setup.ts#L138
import { ApiPromise, WsProvider } from "@polkadot/api";
import child from "child_process";
import * as dotenv from "dotenv";

dotenv.config();

export type ConnectionType = "local" | "binary" | "docker";

interface NodeOpts {
  rpcPort: number;
  dockerOpts: {
    image: string;
    pull: boolean;
  };
  binaryOpts: {
    binaryPath: string;
  };
}

const defaultOpts: NodeOpts = {
  rpcPort: 9944,
  dockerOpts: {
    // image: "ghcr.io/futureversecom/seed:latest",
    image: "seed/pr",
    pull: false,
  },
  binaryOpts: {
    binaryPath: "target/release/seed",
  },
};

export interface NodeProcess {
  id: string;
  wait: () => Promise<void>;
  rpcPort: string;
  stop: () => Promise<unknown>;
}

/**
 * Start a node given connection type
 */
export function startNode(
  type: ConnectionType = (process.env.CONNECTION_TYPE as ConnectionType) ?? "docker",
  nodeOpts?: NodeOpts,
): Promise<NodeProcess> {
  console.info(`Starting node with connection type: ${type}...`);

  // override global console.log to suppress output in CI
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  console.error = (..._args: any[]) => {};

  const nodeOptions = nodeOpts ?? defaultOpts;

  if (type === "local") {
    // connect to an already running node
    const rpcPort = nodeOptions.rpcPort.toString() ?? "9944";
    return Promise.resolve({
      id: "connect",
      rpcPort: rpcPort,
      wait: async () => {
        await ApiPromise.create({ provider: new WsProvider(`ws://127.0.0.1:${rpcPort}`) });
      },
      stop: () => Promise.resolve(),
    });
  }

  if (type === "docker") {
    // start a node in docker
    return startStandaloneDockerNode(nodeOptions);
  }
  if (type === "binary") {
    // TODO integrate startStandaloneNode; path param may be required
    // return startBinaryNode(rpcPort);
    throw new Error(`Unsupported connection type: ${type}`);
  }

  throw new Error(`Unknown connection type: ${type}`);
}

interface DockerInspect {
  NetworkSettings: {
    Ports: {
      "9944/tcp": { HostPort: string }[];
    };
  };
}

async function startStandaloneDockerNode(nodeOpts: NodeOpts): Promise<NodeProcess> {
  const args = [
    "run",
    "--rm",
    "-d", // '-it',
    "-p",
    nodeOpts.rpcPort.toString(),
    "--pull", // image built locally; no need to pull
    "never",
    nodeOpts.dockerOpts.image,
    "--dev",
    "--unsafe-rpc-external",
    "--rpc-port=9944",
    "--rpc-cors=all",
  ];

  // pull the image
  if (nodeOpts.dockerOpts.pull) {
    console.info("pulling image...", nodeOpts.dockerOpts.image);
    await new Promise((resolve, reject) => {
      console.info(`pulling image ${nodeOpts.dockerOpts.image}...`);
      child.exec(`docker pull ${nodeOpts.dockerOpts.image}`, (error, stdout, _) => {
        if (error) {
          reject(error);
        } else {
          resolve(stdout);
        }
      });
    });
  }

  // docker run --platform linux/amd64 --rm -d -p 9944 ghcr.io/futureversecom/seed:latest --dev --tmp --unsafe-rpc-external --rpc-port=9944 --rpc-cors=all
  console.info("starting docker node...\n", "docker", args.join(" "));
  const proc = child.spawn("docker", args);

  // get the docker id from the output
  const id = await new Promise<string>((resolve, reject) => {
    proc.stdout.on("data", (data: unknown) => {
      const id = ((data as any).toString() as string).trim().substring(0, 12);
      resolve(id);
    });
    proc.stderr.on("data", (data: string) => {
      const error = data.toString().trim();
      reject(error);
    });
  });

  // get docker ports - poll at 100ms delay
  const { rpcPort } = await new Promise<{ rpcPort: string }>((resolve, reject) => {
    // let pollCount = 0;
    const interval = setInterval(async () => {
      // console.info(`getting ports for ${id} (${++pollCount})...`);
      child.exec(`docker inspect ${id}`, (error, stdout, _) => {
        clearInterval(interval);
        if (error) {
          return reject(error);
        }
        const inspect: DockerInspect[] = JSON.parse(stdout);
        const ports = inspect[0].NetworkSettings.Ports;
        if (ports["9944/tcp"].length > 0) {
          return resolve({ rpcPort: ports["9944/tcp"][0].HostPort });
        }
      });
    }, 100);
  });
  // console.info(`Docker node started: ${id} - rpc: ${rpcPort}`);

  const stop = () =>
    new Promise((resolve, reject) => {
      // console.info(`stopping docker container ${id}...`);
      child.exec(`docker stop ${id}`, (error, stdout, _) => {
        if (error) {
          console.error(`error stopping docker container ${id}`, error);
          reject(error);
        } else {
          resolve(stdout);
        }
      });
    });

  return {
    id,
    rpcPort,
    wait: async () => {
      await ApiPromise.create({ provider: new WsProvider(`ws://127.0.0.1:${rpcPort}`) });
    },
    stop,
  };
}
