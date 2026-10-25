// Expo's single-page export plus explicit static mobile metadata; no service worker.
import {execFileSync} from 'node:child_process';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
const root=resolve(import.meta.dirname,'..'),client=resolve(root,'apps/client');
execFileSync(resolve(root,'node_modules/.bin/expo'),['export','--platform','web'],{cwd:client,env:process.env,stdio:'inherit'});
const path=resolve(client,'dist/index.html');let html=await readFile(path,'utf8');
if(!html.includes('</head>'))throw new Error('Expo output has no HTML head');
html=html.replace(/<html[^>]*>/,'<html lang="zh-CN">').replace('</head>','<link rel="manifest" href="/manifest.webmanifest"><link rel="icon" href="/icon.svg" type="image/svg+xml"><link rel="apple-touch-icon" href="/icon-192.png"><meta name="apple-mobile-web-app-title" content="skarmar"><meta name="theme-color" content="#34765B"></head>');
await writeFile(path,html);
