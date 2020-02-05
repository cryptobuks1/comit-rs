import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { E2ETestActorConfig } from "../lib/config";
import { waitUntilFileExists } from "./utils";
import * as path from "path";
import lnService from "ln-service";
import { Logger } from "log4js";
import { LogReader } from "../lib/log_reader";
import { mkdirAsync, writeFileAsync } from "./utils";
import { sleep } from "./utils";
import getPort from "get-port";

export class Lnd {
    private process: ChildProcess;
    private lndDir: string;
    private grpc: any;

    constructor(
        private readonly logger: Logger,
        private readonly logDir: string,
        private readonly actorConfig: E2ETestActorConfig,
        private readonly bitcoindDataDir: string
    ) {}

    public async start() {
        const bin = process.env.LND_BIN ? process.env.LND_BIN : "lnd";

        this.logger.debug(`[${this.actorConfig.name}] using binary ${bin}`);

        this.lndDir = path.join(this.logDir, "lnd-" + this.actorConfig.name);
        await mkdirAsync(this.lndDir, "755");
        await this.createConfigFile(this.lndDir);

        this.process = spawn(bin, ["--lnddir", this.lndDir], {
            stdio: ["ignore", "ignore", "ignore"], // stdin, stdout, stderr.  These are all logged already.
        });

        this.logger.debug(
            `[${this.actorConfig.name}] process spawned LND with PID ${this.process.pid}`
        );

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.debug(
                `cnd ${this.actorConfig.name} exited with ${code ||
                    "signal " + signal}`
            );
        });

        this.logger.debug("Waiting for lnd log file to exist:", this.logPath());
        await waitUntilFileExists(this.logPath());

        this.logger.debug("Waiting for lnd password RPC server");
        await this.logReader().waitForLogMessage(
            "RPCS: password RPC server listening"
        );

        const cert = Buffer.from(
            fs.readFileSync(this.tlsCertPath(), "utf8"),
            "utf8"
        ).toString("base64");

        {
            const { lnd } = lnService.unauthenticatedLndGrpc({
                cert,
                socket: this.getGrpcSocket(),
            });
            const { seed } = await lnService.createSeed({ lnd });
            await lnService.createWallet({ lnd, seed, password: "password" });
        }

        this.logger.debug("Waiting for lnd unlocked RPC server");
        await this.logReader().waitForLogMessage("RPCS: RPC server listening");
        this.logger.debug(
            "Waiting for admin macaroon file to exist:",
            this.adminMacaroonPath()
        );
        await waitUntilFileExists(this.adminMacaroonPath());
        const macaroon = fs
            .readFileSync(this.adminMacaroonPath())
            .toString("base64");

        const { lnd } = lnService.authenticatedLndGrpc({
            cert,
            macaroon,
            socket: this.getGrpcSocket(),
        });

        this.grpc = lnd;
        this.logger.debug("Waiting for lnd to catch up with blocks");
        await this.logReader().waitForLogMessage(
            "LNWL: Done catching up block hashes"
        );

        const info = await lnService.getWalletInfo({ lnd: this.grpc });
        this.logger.info("Lnd is ready:", info.public_key);
    }

    public stop() {
        this.process.kill("SIGTERM");
        this.process = null;
    }

    public isRunning() {
        return this.process != null;
    }

    public logPath() {
        return path.join(this.lndDir, "logs", "bitcoin", "regtest", "lnd.log");
    }

    public tlsCertPath() {
        return path.join(this.lndDir, "tls.cert");
    }

    public adminMacaroonPath() {
        return path.join(
            this.lndDir,
            "data",
            "chain",
            "bitcoin",
            "regtest",
            "admin.macaroon"
        );
    }

    public getGrpcSocket() {
        return "127.0.0.1:" + this.actorConfig.lndRpcPort;
    }

    private async dummy() {
        await sleep(1);
    }

    public async fund() {
        await sleep(1);
    }

    public async connect(other: Lnd) {
        await other.dummy();
    }

    public async openChannel(other: Lnd) {
        await other.dummy();
    }

    public async addInvoice(other: Lnd) {
        await other.dummy();
        return "an invoice";
    }

    public async sendPayment(invoice: string) {
        console.log("got invoice: %s", invoice);
        await sleep(1);
    }

    public async assertChannelBalanceSender() {
        await sleep(1);
    }

    public async assertChannelBalanceReceiver() {
        await sleep(1);
    }

    public async assertInvoiceSettled(invoice: string) {
        console.log("got invoice: %s", invoice);
        await sleep(1);
    }

    private async createConfigFile(lndDir: string) {
        // We don't use REST but want a random port so we don't get used port errors.
        const restPort = await getPort();
        const output = `[Application Options]
debuglevel=debug

; peer to peer port
listen=127.0.0.1:${this.actorConfig.lndP2pPort}

; gRPC
rpclisten=127.0.0.1:${this.actorConfig.lndRpcPort}

; REST interface
restlisten=127.0.0.1:${restPort}

; Do not seek out peers on the network
nobootstrap=true

[Bitcoin]

bitcoin.active=true
bitcoin.regtest=true
bitcoin.node=bitcoind

[Bitcoind]

bitcoind.dir=${this.bitcoindDataDir}
`;
        const config = path.join(lndDir, "lnd.conf");
        await writeFileAsync(config, output);
    }

    private logReader() {
        return new LogReader(this.logPath());
    }
}
