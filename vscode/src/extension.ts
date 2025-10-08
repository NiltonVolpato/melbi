import * as path from "path";
import * as fs from "fs";
import { workspace, ExtensionContext } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient;

function getServerPath(context: ExtensionContext): string {
  // In production: use bundled binary
  const bundledPath = path.join(context.extensionPath, "bin", "rhizome-lsp");

  // In development: use workspace binary
  const devPath = path.join(
    context.extensionPath,
    "..",
    "target",
    "debug",
    "rhizome-lsp"
  );

  // Check if we're in development (source folder exists)
  const isDevelopment = fs.existsSync(path.join(context.extensionPath, "src"));

  return isDevelopment ? devPath : bundledPath;
}

export function activate(context: ExtensionContext) {
  const serverExecutable = getServerPath(context);
  if (!fs.existsSync(serverExecutable)) {
    console.error(`Server executable not found: ${serverExecutable}`);
    return;
  }

  const serverOptions: ServerOptions = {
    command: serverExecutable,
    args: [],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "rhizome" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/.rhizome"),
    },
  };

  client = new LanguageClient(
    "rhizome",
    "Rhizome Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
