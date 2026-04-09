import {createGrpcWebTransport} from "@connectrpc/connect-web"
import { Client, createClient } from '@connectrpc/connect';
import type { DescService } from "@bufbuild/protobuf"
/*
 * Usage:
 * default class CodePlaygroundService extends Client.Base<typeof CodePlaygroundClient> {
 *   constructor() {
 *      super(CodePlaygroundClient)
 *   }
 *   ... the rest
 */
export class Base<T extends DescService> {
  client: Client<T>
  constructor(clientDesc: T, baseUrl: string = '/', binaryFormat = true) {
    const transport = createGrpcWebTransport({
      baseUrl: baseUrl || '/',
      useBinaryFormat: true
    })

    this.client = createClient(clientDesc, transport)
  }

  getSecureHeaderFromLocalStorage(tokenName: string): HeadersInit {
    const tokenObj: any = JSON.parse(localStorage.getItem(tokenName) || '{}')
    if (!tokenObj || !tokenObj.content) throw 'Login is required'
    return this.getSecureHeader(tokenObj.content)
  }

  getSecureHeader(token: string): HeadersInit {
    return [
      ['authorization', token]
    ]
  }

  getInSecureHeader(): HeadersInit {
    return []
  }
}
