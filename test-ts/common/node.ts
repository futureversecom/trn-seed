// Adapted with ❤️ from webb-tools - https://github.com/webb-tools/dkg-substrate/blob/0d86b54f57a38881ef0e555ec757b5324e5c8ca7/dkg-test-suite/tests/utils/setup.ts#L138
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

export function startStandaloneNode(
	authority: "alice" | "bob" | "charlie",
	options: { tmp: boolean; printLogs: boolean } = {
		tmp: true,
		printLogs: false,
	}
): child.ChildProcess {
	if (__NODE_STATE[authority].isRunning) {
		return __NODE_STATE[authority].process!;
	}

	const nodePath = "./target/release/dkg-standalone-node";
	const ports = {
		alice: { ws: 9944, http: 9933, p2p: 30333 },
		bob: { ws: 9945, http: 9934, p2p: 30334 },
		charlie: { ws: 9946, http: 9935, p2p: 30335 },
	};
	const proc = child.spawn(nodePath, [
		`--${authority}`,
		options.printLogs ? "-linfo" : "-lerror",
		options.tmp ? `--base-path=./tmp/${authority}` : "",
		`--ws-port=${ports[authority].ws}`,
		`--rpc-port=${ports[authority].http}`,
		`--port=${ports[authority].p2p}`,
		...(authority == "alice"
			? [
					"--node-key",
					"0000000000000000000000000000000000000000000000000000000000000001",
			  ]
			: [
					"--bootnodes",
					`/ip4/127.0.0.1/tcp/${ports["alice"].p2p}/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp`,
			  ]),
		// only print logs from the alice node
		...(authority === "alice" && options.printLogs
			? [
					"-ldkg=debug",
					"-ldkg_metadata=debug",
					"-lruntime::offchain=debug",
					"-ldkg_proposal_handler=debug",
					"--rpc-cors",
					"all",
					"--ws-external",
			  ]
			: []),
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
