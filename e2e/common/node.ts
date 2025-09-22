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
    binaryPath: "target/debug/seed",
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
    return startBinaryNode(nodeOptions);
  }

  throw new Error(`Unknown connection type: ${type}`);
}

interface DockerInspect {
  NetworkSettings: {
    Ports: {
      [portAndProto: string]: { HostPort: string }[] | undefined;
    };
  };
}

async function startStandaloneDockerNode(nodeOpts: NodeOpts): Promise<NodeProcess> {
  function buildArgs(hostPort: number) {
    return [
      "run",
      "--rm",
      "-d",
      "-p",
      `${hostPort}:${nodeOpts.rpcPort}`,
      "--pull",
      "never",
      nodeOpts.dockerOpts.image,
      "--dev",
      "--unsafe-rpc-external",
      `--rpc-port=${nodeOpts.rpcPort}`,
      "--rpc-cors=all",
    ];
  }
  let desiredHostPort = nodeOpts.rpcPort;
  let args = buildArgs(desiredHostPort);

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

  // Example (equivalent) command:
  // docker run --platform linux/amd64 --rm -d -p 9944:9944 ghcr.io/futureversecom/seed:latest --dev --tmp --unsafe-rpc-external --rpc-port=9944 --rpc-cors=all
  console.info("starting docker node...\n", "docker", args.join(" "));
  let proc = child.spawn("docker", args, { stdio: ["ignore", "pipe", "pipe"] });

  // Capture stdout/stderr to parse container id after process spawn returns output.
  let attemptedDynamicPort = false;
  const id = await new Promise<string>((resolve, reject) => {
    let stdoutBuf = "";
    let stderrBuf = "";
    proc.stdout.on("data", (data: Buffer) => {
      stdoutBuf += data.toString();
    });
    proc.stderr.on("data", (data: Buffer) => {
      stderrBuf += data.toString();
    });
    proc.on("error", (err) => reject(err));
    proc.on("close", (code) => {
      if (code !== 0) {
        // Retry once with a random high port if bind failed
        if (!attemptedDynamicPort && /port is already allocated/i.test(stderrBuf)) {
          attemptedDynamicPort = true;
          desiredHostPort = 10_000 + Math.floor(Math.random() * 50_000); // ephemeral
          args = buildArgs(desiredHostPort);
          console.info(`Port ${nodeOpts.rpcPort} busy; retrying with host port ${desiredHostPort}`);
          proc = child.spawn("docker", args, { stdio: ["ignore", "pipe", "pipe"] });
          stdoutBuf = "";
          stderrBuf = "";
          proc.stdout.on("data", (d: Buffer) => {
            stdoutBuf += d.toString();
          });
          proc.stderr.on("data", (d: Buffer) => {
            stderrBuf += d.toString();
          });
          proc.on("error", (err) => reject(err));
          proc.on("close", (code2) => {
            if (code2 !== 0) {
              return reject(new Error(`docker run retry exited with code ${code2}: ${stderrBuf || stdoutBuf}`));
            }
            const raw2 = stdoutBuf.trim().split(/\s+/)[0];
            if (!raw2 || raw2.length < 12) {
              return reject(
                new Error(`Failed to capture docker container id (retry). stdout='${stdoutBuf}' stderr='${stderrBuf}'`),
              );
            }
            resolve(raw2.substring(0, 12));
          });
          return; // handled retry
        }
        return reject(new Error(`docker run exited with code ${code}: ${stderrBuf || stdoutBuf}`));
      }
      const raw = stdoutBuf.trim().split(/\s+/)[0];
      if (!raw || raw.length < 12) {
        return reject(new Error(`Failed to capture docker container id. stdout='${stdoutBuf}' stderr='${stderrBuf}'`));
      }
      resolve(raw.substring(0, 12));
    });
  });

  // get docker ports - poll at 100ms delay
  const { rpcPort } = await new Promise<{ rpcPort: string }>((resolve, reject) => {
    const target = `${nodeOpts.rpcPort}/tcp`;
    let attempts = 0;
    const maxAttempts = 200; // 20s @ 100ms
    const interval = setInterval(() => {
      attempts++;
      child.exec(`docker inspect ${id}`, (error, stdout, _) => {
        if (error) {
          clearInterval(interval);
          return reject(error);
        }
        try {
          const inspect: DockerInspect[] = JSON.parse(stdout);
          const ports = inspect[0].NetworkSettings.Ports;
          // If we retried with a different host port, the container still exposes container port nodeOpts.rpcPort
          // but host port may differ; search for the mapping whose key matches container port.
          const mapping = ports[target];
          if (mapping && mapping.length > 0 && mapping[0].HostPort) {
            clearInterval(interval);
            return resolve({ rpcPort: mapping[0].HostPort });
          }
          if (attempts >= maxAttempts) {
            clearInterval(interval);
            return reject(new Error(`Timed out waiting for docker port mapping for ${target}`));
          }
        } catch (e) {
          clearInterval(interval);
          return reject(e);
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
      // First poll HTTP JSON-RPC for readiness (faster + avoids WS hang edge cases)
      const maxMs = 60_000;
      const start = Date.now();
      while (Date.now() - start < maxMs) {
        const ok = await new Promise<boolean>((resolve) => {
          child.exec(
            `curl -s -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' http://127.0.0.1:${rpcPort}`,
            (err, stdout) => {
              if (err) return resolve(false);
              if (stdout.includes("isSyncing")) return resolve(true);
              resolve(false);
            },
          );
        });
        if (ok) break;
        await new Promise((r) => setTimeout(r, 500));
      }
      // Final attempt to establish WS (ensures downstream ApiPromise usage succeeds quickly)
      await ApiPromise.create({ provider: new WsProvider(`ws://127.0.0.1:${rpcPort}`) });
    },
    stop,
  };
}

async function startBinaryNode(nodeOpts: NodeOpts): Promise<NodeProcess> {
  const rpcPort = nodeOpts.rpcPort.toString();
  // Resolve binary path relative to project root (this file sits in e2e/common)
  const path = nodeOpts.binaryOpts.binaryPath.startsWith("/")
    ? nodeOpts.binaryOpts.binaryPath
    : require("path").join(__dirname, "../../", nodeOpts.binaryOpts.binaryPath);
  const args = ["--dev", "--unsafe-rpc-external", `--rpc-port=${rpcPort}`, "--rpc-cors=all"];
  console.info("starting local binary node...", path, args.join(" "));
  const proc = child.spawn(path, args, { stdio: ["ignore", "pipe", "pipe"] });
  let exited = false;
  proc.on("exit", (code, signal) => {
    exited = true;
    console.error(`local node exited code=${code} signal=${signal}`);
  });
  const wait = async () => {
    if (exited) throw new Error("node process exited early");
    const maxMs = 30_000;
    const start = Date.now();
    while (Date.now() - start < maxMs) {
      const ok = await new Promise<boolean>((resolve) => {
        child.exec(
          `curl -s -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' http://127.0.0.1:${rpcPort}`,
          (err, stdout) => {
            if (err) return resolve(false);
            if (stdout.includes("isSyncing")) return resolve(true);
            resolve(false);
          },
        );
      });
      if (ok) {
        await ApiPromise.create({ provider: new WsProvider(`ws://127.0.0.1:${rpcPort}`) });
        return;
      }
      await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error("Timed out waiting for local binary node to be ready");
  };
  const stop = async () => {
    proc.kill();
  };
  return { id: `binary-${proc.pid}`, rpcPort, wait, stop };
}
