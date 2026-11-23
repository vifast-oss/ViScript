// SPDX-License-Identifier: MIT OR Apache-2.0
// **ST-305**: extension mỏng — chỉ khởi động `vis lsp --stdio` và nối JSON-RPC.
// Không build/publish trong repo; cần `npm install` trong editors/vscode để có
// `vscode-languageclient`.
const { workspace } = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client;

function activate(context) {
  const command = workspace.getConfiguration('viscript').get('serverPath') || 'vis';
  const serverOptions = {
    command,
    args: ['lsp', '--stdio'],
    transport: TransportKind.stdio,
  };
  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'viscript' }],
  };
  client = new LanguageClient('viscript', 'ViScript Language Server', serverOptions, clientOptions);
  context.subscriptions.push(client.start());
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
