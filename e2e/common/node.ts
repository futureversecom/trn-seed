// Adapted  from webb-tools - https://github.com/webb-tools/dkg-substrate/blob/0d86b54f57a38881ef0e555ec757b5324e5c8ca7/dkg-test-suite/tests/utils/setup.ts#L138
import { ApiPromise, WsProvider } from "@polkadot/api";
import child from "child_process";

// a global variable to check if the node is already running or not.
// to avoid running multiple nodes with the same authority at the same time.
const __NODE_STATE: {
  [authorityId: string]: {
    process: child.ChildProcess | null;
    isRunning: boolean;
  };
} = {
  alice: { isRunning: false, process: null },
  bob: { isRunning: false, process: null },
  charlie: { isRunning: false, process: null },
};

export type ConnectionType = "local" | "binary" | "docker";

interface NodeOpts {
  type: ConnectionType;
  httpPort: number;
  wsPort: number;
  dockerOpts: {
    image: string;
    pull: boolean;
  };
  binaryOpts: {
    binaryPath: string;
  };
}

const defaultDockerOpts: NodeOpts = {
  type: "docker",
  httpPort: 9933,
  wsPort: 9944,
  dockerOpts: {
    // image: "ghcr.io/futureversecom/seed:latest",
    image: "seed/pr",
    pull: false,
  },
  binaryOpts: {
    binaryPath: "target/release/seed",
  },
};

const defaultLocalOpts: NodeOpts = {
  type: "local",
  httpPort: 9933,
  wsPort: 9944,
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
  httpPort: string;
  wsPort: string;
  stop: () => Promise<unknown>;
}

/**
 * Start a node given connection type
 */
export function startNode(nodeOpts?: NodeOpts): Promise<NodeProcess> {
  // Start local options if the NODE_OPTION is set to "local"
  const defaultOpts = process.env.NODE_OPTION == "local" ? defaultLocalOpts : defaultDockerOpts;
  const nodeOptions = nodeOpts ?? defaultOpts;

  if (nodeOptions.type === "local") {
    // connect to an already running node
    const wsPortStr = nodeOptions.wsPort.toString() ?? "9944";
    return Promise.resolve({
      id: "connect",
      httpPort: nodeOptions.httpPort.toString() ?? "9933",
      wsPort: wsPortStr,
      wait: async () => {
        await ApiPromise.create({ provider: new WsProvider(`ws://localhost:${wsPortStr}`) });
      },
      stop: () => Promise.resolve(),
    });
  }

  if (nodeOptions.type === "docker") {
    // start a node in docker
    return startStandaloneDockerNode(nodeOptions);
  }
  if (nodeOptions.type === "binary") {
    // TODO integrate startStandaloneNode; path param may be required
    // return startBinaryNode(httpPort, wsPort);
    throw new Error(`Unsupported connection type: ${nodeOptions.type}`);
  }

  throw new Error(`Unknown connection type: ${nodeOptions.type}`);
}

interface DockerInspect {
  NetworkSettings: {
    Ports: {
      "9944/tcp": { HostPort: string }[];
      "9933/tcp": { HostPort: string }[];
    };
  };
}

async function startStandaloneDockerNode(nodeOpts: NodeOpts): Promise<NodeProcess> {
  const args = [
    "run",
    "--rm",
    "-d", // '-it',
    "-p",
    nodeOpts.httpPort.toString(),
    "-p",
    nodeOpts.wsPort.toString(),
    "--pull", // image built locally; no need to pull
    "never",
    nodeOpts.dockerOpts.image,
    "--dev",
    "--unsafe-ws-external",
    "--unsafe-rpc-external",
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

  // docker run --platform linux/amd64 --rm -d -p 9933 -p 9944 ghcr.io/futureversecom/seed:latest --dev --tmp --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all
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
  const { httpPort, wsPort } = await new Promise<{ httpPort: string; wsPort: string }>((resolve, reject) => {
    let pollCount = 0;
    const interval = setInterval(async () => {
      console.info(`getting ports for ${id} (${++pollCount})...`);
      child.exec(`docker inspect ${id}`, (error, stdout, _) => {
        clearInterval(interval);
        if (error) {
          return reject(error);
        }
        const inspect: DockerInspect[] = JSON.parse(stdout);
        const ports = inspect[0].NetworkSettings.Ports;
        if (ports["9933/tcp"].length > 0 && ports["9944/tcp"].length > 0) {
          return resolve({ httpPort: ports["9933/tcp"][0].HostPort, wsPort: ports["9944/tcp"][0].HostPort });
        }
      });
    }, 100);
  });
  // console.info(`Docker node started: ${id} - http: ${httpPort} - ws: ${wsPort}`);

  const stop = () =>
    new Promise((resolve, reject) => {
      console.info(`stopping docker container ${id}...`);
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
    httpPort,
    wsPort,
    wait: async () => {
      await ApiPromise.create({ provider: new WsProvider(`ws://localhost:${wsPort}`) });
    },
    stop,
  };
}

export function startStandaloneNode(
  authority: "alice" | "bob" | "charlie",
  options: { tmp: boolean; printLogs: boolean } = {
    tmp: true,
    printLogs: false,
  },
): child.ChildProcess {
  if (__NODE_STATE[authority].isRunning) {
    return __NODE_STATE[authority].process!;
  }

  const nodePath = "../target/release/seed";
  const ports = {
    alice: { ws: 9944, http: 9933, p2p: 30333 },
    bob: { ws: 9945, http: 9934, p2p: 30334 },
    charlie: { ws: 9946, http: 9935, p2p: 30335 },
  };

  const aliceNodeId = "12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp";

  const proc = child.spawn(nodePath, [
    "--dev",
    options.printLogs ? "-linfo" : "-lerror",
    `--ws-port=${ports[authority].ws}`,
    `--rpc-port=${ports[authority].http}`,
    `--port=${ports[authority].p2p}`,
    ...(authority == "alice"
      ? ["--node-key", "0000000000000000000000000000000000000000000000000000000000000001"]
      : ["--bootnodes", `/ip4/127.0.0.1/tcp/${ports["alice"].p2p}/p2p/${aliceNodeId}`]),
    // only print logs from the alice node
    ...(authority === "alice" && options.printLogs ? ["--rpc-cors", "all", "--ws-external"] : []),
  ]);
  __NODE_STATE[authority].isRunning = true;
  __NODE_STATE[authority].process = proc;

  proc.stdout.on("data", (data) => {
    process.stdout.write(data);
  });

  proc.stderr.on("data", (data) => {
    process.stdout.write(data);
  });

  proc.on("close", (code) => {
    __NODE_STATE[authority].isRunning = false;
    __NODE_STATE[authority].process = null;
    console.log(`${authority} node exited with code ${code}`);
  });

  return proc;
}
