let wasm_bindgen;
(function() {
    const __exports = {};
    let script_src;
    if (typeof document !== 'undefined' && document.currentScript !== null) {
        script_src = new URL(document.currentScript.src, location.href).toString();
    }
    let wasm = undefined;

    let cachedUint8ArrayMemory0 = null;

    function getUint8ArrayMemory0() {
        if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
            cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
        }
        return cachedUint8ArrayMemory0;
    }

    let cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

    if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

    function decodeText(ptr, len) {
        return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
    }

    function getStringFromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return decodeText(ptr, len);
    }

    function logError(f, args) {
        try {
            return f.apply(this, args);
        } catch (e) {
            let error = (function () {
                try {
                    return e instanceof Error ? `${e.message}\n\nStack:\n${e.stack}` : e.toString();
                } catch(_) {
                    return "<failed to stringify thrown value>";
                }
            }());
            console.error("wasm-bindgen: imported JS function that was not marked as `catch` threw an error:", error);
            throw e;
        }
    }

    function isLikeNone(x) {
        return x === undefined || x === null;
    }

    function addToExternrefTable0(obj) {
        const idx = wasm.__externref_table_alloc();
        wasm.__wbindgen_export_1.set(idx, obj);
        return idx;
    }

    function _assertNum(n) {
        if (typeof(n) !== 'number') throw new Error(`expected a number argument, found ${typeof(n)}`);
    }

    function handleError(f, args) {
        try {
            return f.apply(this, args);
        } catch (e) {
            const idx = addToExternrefTable0(e);
            wasm.__wbindgen_exn_store(idx);
        }
    }

    let WASM_VECTOR_LEN = 0;

    const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

    const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
        ? function (arg, view) {
        return cachedTextEncoder.encodeInto(arg, view);
    }
        : function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    });

    function passStringToWasm0(arg, malloc, realloc) {

        if (typeof(arg) !== 'string') throw new Error(`expected a string argument, found ${typeof(arg)}`);

        if (realloc === undefined) {
            const buf = cachedTextEncoder.encode(arg);
            const ptr = malloc(buf.length, 1) >>> 0;
            getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
            WASM_VECTOR_LEN = buf.length;
            return ptr;
        }

        let len = arg.length;
        let ptr = malloc(len, 1) >>> 0;

        const mem = getUint8ArrayMemory0();

        let offset = 0;

        for (; offset < len; offset++) {
            const code = arg.charCodeAt(offset);
            if (code > 0x7F) break;
            mem[ptr + offset] = code;
        }

        if (offset !== len) {
            if (offset !== 0) {
                arg = arg.slice(offset);
            }
            ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
            const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
            const ret = encodeString(arg, view);
            if (ret.read !== arg.length) throw new Error('failed to pass whole string');
            offset += ret.written;
            ptr = realloc(ptr, len, offset, 1) >>> 0;
        }

        WASM_VECTOR_LEN = offset;
        return ptr;
    }

    let cachedDataViewMemory0 = null;

    function getDataViewMemory0() {
        if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
            cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
        }
        return cachedDataViewMemory0;
    }

    function _assertBoolean(n) {
        if (typeof(n) !== 'boolean') {
            throw new Error(`expected a boolean argument, found ${typeof(n)}`);
        }
    }

    function getArrayU8FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
    }

    function _assertBigInt(n) {
        if (typeof(n) !== 'bigint') throw new Error(`expected a bigint argument, found ${typeof(n)}`);
    }

    const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(state => {
        wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b)
    });

    function makeMutClosure(arg0, arg1, dtor, f) {
        const state = { a: arg0, b: arg1, cnt: 1, dtor };
        const real = (...args) => {
            // First up with a closure we increment the internal reference
            // count. This ensures that the Rust closure environment won't
            // be deallocated while we're invoking it.
            state.cnt++;
            const a = state.a;
            state.a = 0;
            try {
                return f(a, state.b, ...args);
            } finally {
                if (--state.cnt === 0) {
                    wasm.__wbindgen_export_6.get(state.dtor)(a, state.b);
                    CLOSURE_DTORS.unregister(state);
                } else {
                    state.a = a;
                }
            }
        };
        real.original = state;
        CLOSURE_DTORS.register(real, state, state);
        return real;
    }

    function makeClosure(arg0, arg1, dtor, f) {
        const state = { a: arg0, b: arg1, cnt: 1, dtor };
        const real = (...args) => {
            // First up with a closure we increment the internal reference
            // count. This ensures that the Rust closure environment won't
            // be deallocated while we're invoking it.
            state.cnt++;
            try {
                return f(state.a, state.b, ...args);
            } finally {
                if (--state.cnt === 0) {
                    wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b);
                    state.a = 0;
                    CLOSURE_DTORS.unregister(state);
                }
            }
        };
        real.original = state;
        CLOSURE_DTORS.register(real, state, state);
        return real;
    }

    function debugString(val) {
        // primitive types
        const type = typeof val;
        if (type == 'number' || type == 'boolean' || val == null) {
            return  `${val}`;
        }
        if (type == 'string') {
            return `"${val}"`;
        }
        if (type == 'symbol') {
            const description = val.description;
            if (description == null) {
                return 'Symbol';
            } else {
                return `Symbol(${description})`;
            }
        }
        if (type == 'function') {
            const name = val.name;
            if (typeof name == 'string' && name.length > 0) {
                return `Function(${name})`;
            } else {
                return 'Function';
            }
        }
        // objects
        if (Array.isArray(val)) {
            const length = val.length;
            let debug = '[';
            if (length > 0) {
                debug += debugString(val[0]);
            }
            for(let i = 1; i < length; i++) {
                debug += ', ' + debugString(val[i]);
            }
            debug += ']';
            return debug;
        }
        // Test for built-in
        const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
        let className;
        if (builtInMatches && builtInMatches.length > 1) {
            className = builtInMatches[1];
        } else {
            // Failed to match the standard '[object ClassName]'
            return toString.call(val);
        }
        if (className == 'Object') {
            // we're a user defined class or Object
            // JSON.stringify avoids problems with cycles, and is generally much
            // easier than looping through ownProperties of `val`.
            try {
                return 'Object(' + JSON.stringify(val) + ')';
            } catch (_) {
                return 'Object';
            }
        }
        // errors
        if (val instanceof Error) {
            return `${val.name}: ${val.message}\n${val.stack}`;
        }
        // TODO we could test for more things here, like `Set`s and `Map`s.
        return className;
    }
    /**
     * @returns {Promise<void>}
     */
    __exports.start_worker = function() {
        wasm.start_worker();
    };

    /**
     * @param {Uint8Array} data
     * @returns {Promise<Uint8Array>}
     */
    __exports.process_event = function(data) {
        const ret = wasm.process_event(data);
        return ret;
    };

    /**
     * @param {number} id
     * @param {Uint8Array} data
     * @returns {Promise<Uint8Array>}
     */
    __exports.handle_response = function(id, data) {
        _assertNum(id);
        const ret = wasm.handle_response(id, data);
        return ret;
    };

    /**
     * @returns {Promise<Uint8Array>}
     */
    __exports.view = function() {
        const ret = wasm.view();
        return ret;
    };

    /**
     * @returns {Promise<void>}
     */
    __exports.init = function() {
        const ret = wasm.init();
        return ret;
    };

    /**
     * @param {Array<any>} files
     * @returns {Promise<Uint8Array>}
     */
    __exports.add_device_files = function(files) {
        const ret = wasm.add_device_files(files);
        return ret;
    };

    /**
     * @param {bigint} resource_id
     * @returns {Promise<File | undefined>}
     */
    __exports.get_device_file = function(resource_id) {
        _assertBigInt(resource_id);
        const ret = wasm.get_device_file(resource_id);
        return ret;
    };

    /**
     * @param {bigint} resource_id
     * @returns {Promise<Uint8Array | undefined>}
     */
    __exports.load_thumbnail_bytes = function(resource_id) {
        _assertBigInt(resource_id);
        const ret = wasm.load_thumbnail_bytes(resource_id);
        return ret;
    };

    /**
     * @param {Uint8Array} path
     * @returns {Promise<string | undefined>}
     */
    __exports.load_thumbnail_source = function(path) {
        const ret = wasm.load_thumbnail_source(path);
        return ret;
    };

    /**
     * @param {Uint8Array} path
     * @param {FileSystemWritableFileStream} writer
     * @returns {Promise<void>}
     */
    __exports.download_file_from_cache = function(path, writer) {
        const ret = wasm.download_file_from_cache(path, writer);
        return ret;
    };

    /**
     * @param {number} request_id
     * @param {Uint8Array} effect
     * @returns {Promise<Uint8Array>}
     */
    __exports.execute = function(request_id, effect) {
        _assertNum(request_id);
        const ret = wasm.execute(request_id, effect);
        return ret;
    };

    /**
     * @returns {Promise<boolean>}
     */
    __exports.is_compatible = function() {
        const ret = wasm.is_compatible();
        return ret;
    };

    function __wbg_adapter_50(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure113_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_53(arg0, arg1) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.wasm_bindgen__convert__closures_____invoke__h933ca4df983b9f21(arg0, arg1);
    }

    function __wbg_adapter_56(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure6705_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_209(arg0, arg1, arg2, arg3) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure6712_externref_shim(arg0, arg1, arg2, arg3);
    }

    const __wbindgen_enum_ReadableStreamType = ["bytes"];

    const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0, 1));

    class IntoUnderlyingByteSource {

        constructor() {
            throw new Error('cannot invoke `new` directly');
        }

        __destroy_into_raw() {
            const ptr = this.__wbg_ptr;
            this.__wbg_ptr = 0;
            IntoUnderlyingByteSourceFinalization.unregister(this);
            return ptr;
        }

        free() {
            const ptr = this.__destroy_into_raw();
            wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
        }
        /**
         * @returns {ReadableStreamType}
         */
        get type() {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
            return __wbindgen_enum_ReadableStreamType[ret];
        }
        /**
         * @returns {number}
         */
        get autoAllocateChunkSize() {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
            return ret >>> 0;
        }
        /**
         * @param {ReadableByteStreamController} controller
         */
        start(controller) {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            wasm.intounderlyingbytesource_start(this.__wbg_ptr, controller);
        }
        /**
         * @param {ReadableByteStreamController} controller
         * @returns {Promise<any>}
         */
        pull(controller) {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, controller);
            return ret;
        }
        cancel() {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            const ptr = this.__destroy_into_raw();
            _assertNum(ptr);
            wasm.intounderlyingbytesource_cancel(ptr);
        }
    }
    __exports.IntoUnderlyingByteSource = IntoUnderlyingByteSource;

    const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0, 1));

    class IntoUnderlyingSink {

        constructor() {
            throw new Error('cannot invoke `new` directly');
        }

        __destroy_into_raw() {
            const ptr = this.__wbg_ptr;
            this.__wbg_ptr = 0;
            IntoUnderlyingSinkFinalization.unregister(this);
            return ptr;
        }

        free() {
            const ptr = this.__destroy_into_raw();
            wasm.__wbg_intounderlyingsink_free(ptr, 0);
        }
        /**
         * @param {any} chunk
         * @returns {Promise<any>}
         */
        write(chunk) {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, chunk);
            return ret;
        }
        /**
         * @returns {Promise<any>}
         */
        close() {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            const ptr = this.__destroy_into_raw();
            _assertNum(ptr);
            const ret = wasm.intounderlyingsink_close(ptr);
            return ret;
        }
        /**
         * @param {any} reason
         * @returns {Promise<any>}
         */
        abort(reason) {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            const ptr = this.__destroy_into_raw();
            _assertNum(ptr);
            const ret = wasm.intounderlyingsink_abort(ptr, reason);
            return ret;
        }
    }
    __exports.IntoUnderlyingSink = IntoUnderlyingSink;

    const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0, 1));

    class IntoUnderlyingSource {

        constructor() {
            throw new Error('cannot invoke `new` directly');
        }

        __destroy_into_raw() {
            const ptr = this.__wbg_ptr;
            this.__wbg_ptr = 0;
            IntoUnderlyingSourceFinalization.unregister(this);
            return ptr;
        }

        free() {
            const ptr = this.__destroy_into_raw();
            wasm.__wbg_intounderlyingsource_free(ptr, 0);
        }
        /**
         * @param {ReadableStreamDefaultController} controller
         * @returns {Promise<any>}
         */
        pull(controller) {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, controller);
            return ret;
        }
        cancel() {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            const ptr = this.__destroy_into_raw();
            _assertNum(ptr);
            wasm.intounderlyingsource_cancel(ptr);
        }
    }
    __exports.IntoUnderlyingSource = IntoUnderlyingSource;

    const EXPECTED_RESPONSE_TYPES = new Set(['basic', 'cors', 'default']);

    async function __wbg_load(module, imports) {
        if (typeof Response === 'function' && module instanceof Response) {
            if (typeof WebAssembly.instantiateStreaming === 'function') {
                try {
                    return await WebAssembly.instantiateStreaming(module, imports);

                } catch (e) {
                    const validResponse = module.ok && EXPECTED_RESPONSE_TYPES.has(module.type);

                    if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                        console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                    } else {
                        throw e;
                    }
                }
            }

            const bytes = await module.arrayBuffer();
            return await WebAssembly.instantiate(bytes, imports);

        } else {
            const instance = await WebAssembly.instantiate(module, imports);

            if (instance instanceof WebAssembly.Instance) {
                return { instance, module };

            } else {
                return instance;
            }
        }
    }

    function __wbg_get_imports() {
        const imports = {};
        imports.wbg = {};
        imports.wbg.__wbg_Error_0497d5bdba9362e5 = function() { return logError(function (arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_buffer_a1a27a0dfa70165d = function() { return logError(function (arg0) {
            const ret = arg0.buffer;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_buffer_e495ba54cee589cc = function() { return logError(function (arg0) {
            const ret = arg0.buffer;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_byobRequest_56aa768ee4dfed17 = function() { return logError(function (arg0) {
            const ret = arg0.byobRequest;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_byteLength_937f8a52f9697148 = function() { return logError(function (arg0) {
            const ret = arg0.byteLength;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_byteOffset_4d94b7170e641898 = function() { return logError(function (arg0) {
            const ret = arg0.byteOffset;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_caches_94f0a9684564dc48 = function() { return handleError(function (arg0) {
            const ret = arg0.caches;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_call_f2db6205e5c51dc8 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_call_fbe8be8bf6436ce5 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_clearTimeout_96804de0ab838f26 = function() { return logError(function (arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_close_290fb040af98d3ac = function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_close_48dd1e5910dd5cf9 = function() { return logError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_close_b2641ef0870e518c = function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_createObjectURL_1acd82bf8749f5a9 = function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_data_fffd43bf0ca75fff = function() { return logError(function (arg0) {
            const ret = arg0.data;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_done_4d01f352bade43b7 = function() { return logError(function (arg0) {
            const ret = arg0.done;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_enqueue_a62faa171c4fd287 = function() { return handleError(function (arg0, arg1) {
            arg0.enqueue(arg1);
        }, arguments) };
        imports.wbg.__wbg_entries_41651c850143b957 = function() { return logError(function (arg0) {
            const ret = Object.entries(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_error_7534b8e9a36f1ab4 = function() { return logError(function (arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        }, arguments) };
        imports.wbg.__wbg_estimate_1f4d5dcdcb6644e9 = function() { return handleError(function (arg0) {
            const ret = arg0.estimate();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_forwardcoreoperationoutput_484b25b51e680571 = function() { return logError(function (arg0, arg1) {
            const ret = core.forward_core_operation_output(arg0 >>> 0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getRandomValues_e14bd3de0db61032 = function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments) };
        imports.wbg.__wbg_getTime_2afe67905d873e92 = function() { return logError(function (arg0) {
            const ret = arg0.getTime();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getTimezoneOffset_31f33c0868da345e = function() { return logError(function (arg0) {
            const ret = arg0.getTimezoneOffset();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_get_92470be87867c2e5 = function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_get_a131a44bd1eb6979 = function() { return logError(function (arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getwithrefkey_1dc361bd10053bfe = function() { return logError(function (arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        }, arguments) };
        imports.wbg.__wbg_href_9bc8ec9ea5cd0919 = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_instanceof_ArrayBuffer_a8b6f580b363f2bc = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_File_fce38dde217890fd = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof File;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_Uint8Array_ca460677bc155827 = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_Window_68f3f67bad1729c1 = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_isArray_5f090bed72bd4f89 = function() { return logError(function (arg0) {
            const ret = Array.isArray(arg0);
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_isSafeInteger_90d7c4674047d684 = function() { return logError(function (arg0) {
            const ret = Number.isSafeInteger(arg0);
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_iterator_4068add5b2aef7a6 = function() { return logError(function () {
            const ret = Symbol.iterator;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_length_ab6d22b5ead75c72 = function() { return logError(function (arg0) {
            const ret = arg0.length;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_length_f00ec12454a5d9fd = function() { return logError(function (arg0) {
            const ret = arg0.length;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_location_53fd1bbab625ae8d = function() { return logError(function (arg0) {
            const ret = arg0.location;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_log_ea240990d83e374e = function() { return logError(function (arg0) {
            console.log(arg0);
        }, arguments) };
        imports.wbg.__wbg_navigator_fc64ba1417939b25 = function() { return logError(function (arg0) {
            const ret = arg0.navigator;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new0_97314565408dea38 = function() { return logError(function () {
            const ret = new Date();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_07b483f72211fd66 = function() { return logError(function () {
            const ret = new Object();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_39fae4e38868373c = function() { return handleError(function (arg0, arg1) {
            const ret = new Worker(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_476169e6d59f23ae = function() { return logError(function (arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_58353953ad2097cc = function() { return logError(function () {
            const ret = new Array();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_8a6f238a6ece86ea = function() { return logError(function () {
            const ret = new Error();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_a2957aa5684de228 = function() { return logError(function (arg0) {
            const ret = new Date(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_e30c39c06edaabf2 = function() { return logError(function (arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return __wbg_adapter_209(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = state0.b = 0;
            }
        }, arguments) };
        imports.wbg.__wbg_new_e52b3efaaa774f96 = function() { return logError(function (arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newfromslice_7c05ab1297cb2d88 = function() { return logError(function (arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newnoargs_ff528e72d35de39a = function() { return logError(function (arg0, arg1) {
            const ret = new Function(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithbase_97f2404ce617ec5a = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = new URL(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithbyteoffsetandlength_3b01ecda099177e8 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithstrsequenceandoptions_3c68d739cf8f35ce = function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_next_8bb824d217961b5d = function() { return logError(function (arg0) {
            const ret = arg0.next;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_next_e2da48d8fff7439a = function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_now_6af59e24f5a53ad4 = function() { return handleError(function () {
            const ret = Date.now();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_postMessage_54ce7f4b41ac732e = function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments) };
        imports.wbg.__wbg_postMessage_95ef4554c6b7ca0c = function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments) };
        imports.wbg.__wbg_push_73fd7b5550ebf707 = function() { return logError(function (arg0, arg1) {
            const ret = arg0.push(arg1);
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_queueMicrotask_46c1df247678729f = function() { return logError(function (arg0) {
            queueMicrotask(arg0);
        }, arguments) };
        imports.wbg.__wbg_queueMicrotask_8acf3ccb75ed8d11 = function() { return logError(function (arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_replace_fe93a07a97dbbd6c = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.replace(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_resolve_0dac8c580ffd4678 = function() { return logError(function (arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_respond_b227f1c3be2bb879 = function() { return handleError(function (arg0, arg1) {
            arg0.respond(arg1 >>> 0);
        }, arguments) };
        imports.wbg.__wbg_setTimeout_eefe7f4c234b0c6b = function() { return handleError(function (arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_set_3f1d0b984ed272ed = function() { return logError(function (arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        }, arguments) };
        imports.wbg.__wbg_set_7422acbe992d64ab = function() { return logError(function (arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        }, arguments) };
        imports.wbg.__wbg_set_fe4e79d1ed3b0e9b = function() { return logError(function (arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        }, arguments) };
        imports.wbg.__wbg_setonmessage_36f21db011898669 = function() { return logError(function (arg0, arg1) {
            arg0.onmessage = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonmessage_f6cf46183c427754 = function() { return logError(function (arg0, arg1) {
            arg0.onmessage = arg1;
        }, arguments) };
        imports.wbg.__wbg_settype_acc38e64fddb9e3f = function() { return logError(function (arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        }, arguments) };
        imports.wbg.__wbg_stack_0ed75d68575b0f3c = function() { return logError(function (arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_GLOBAL_487c52c58d65314d = function() { return logError(function () {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_GLOBAL_THIS_ee9704f328b6b291 = function() { return logError(function () {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_SELF_78c9e3071b912620 = function() { return logError(function () {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_WINDOW_a093d21393777366 = function() { return logError(function () {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_storage_fb48e219fca48755 = function() { return logError(function (arg0) {
            const ret = arg0.storage;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_then_82ab9fb4080f1707 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_then_db882932c0c714c6 = function() { return logError(function (arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_toISOString_ee664672ac17246b = function() { return logError(function (arg0) {
            const ret = arg0.toISOString();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_toString_bc7a05a172b5cf14 = function() { return logError(function (arg0) {
            const ret = arg0.toString();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_updateappevent_b051ac6eba9da569 = function() { return logError(function (arg0) {
            const ret = core.update_app_event(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_value_17b896954e14f896 = function() { return logError(function (arg0) {
            const ret = arg0.value;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_view_a9ad80dcbad7cf1c = function() { return logError(function (arg0) {
            const ret = arg0.view;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbindgen_as_number = function(arg0) {
            const ret = +arg0;
            return ret;
        };
        imports.wbg.__wbindgen_bigint_from_u64 = function(arg0) {
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        };
        imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            if (!isLikeNone(ret)) {
                _assertBigInt(ret);
            }
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        };
        imports.wbg.__wbindgen_boolean_get = function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
            _assertNum(ret);
            return ret;
        };
        imports.wbg.__wbindgen_cb_drop = function(arg0) {
            const obj = arg0.original;
            if (obj.cnt-- == 1) {
                obj.a = 0;
                return true;
            }
            const ret = false;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper27746 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 6645, __wbg_adapter_53);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_closure_wrapper28781 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 6704, __wbg_adapter_56);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_closure_wrapper931 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = makeClosure(arg0, arg1, 112, __wbg_adapter_50);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbindgen_in = function(arg0, arg1) {
            const ret = arg0 in arg1;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_init_externref_table = function() {
            const table = wasm.__wbindgen_export_1;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
            ;
        };
        imports.wbg.__wbindgen_is_bigint = function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_is_function = function(arg0) {
            const ret = typeof(arg0) === 'function';
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_is_null = function(arg0) {
            const ret = arg0 === null;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_is_object = function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_is_string = function(arg0) {
            const ret = typeof(arg0) === 'string';
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_is_undefined = function(arg0) {
            const ret = arg0 === undefined;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
            const ret = arg0 === arg1;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
            const ret = arg0 == arg1;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbindgen_memory = function() {
            const ret = wasm.memory;
            return ret;
        };
        imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            if (!isLikeNone(ret)) {
                _assertNum(ret);
            }
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        };
        imports.wbg.__wbindgen_number_new = function(arg0) {
            const ret = arg0;
            return ret;
        };
        imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        };
        imports.wbg.__wbindgen_throw = function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        };

        return imports;
    }

    function __wbg_init_memory(imports, memory) {

    }

    function __wbg_finalize_init(instance, module) {
        wasm = instance.exports;
        __wbg_init.__wbindgen_wasm_module = module;
        cachedDataViewMemory0 = null;
        cachedUint8ArrayMemory0 = null;


        wasm.__wbindgen_start();
        return wasm;
    }

    function initSync(module) {
        if (wasm !== undefined) return wasm;


        if (typeof module !== 'undefined') {
            if (Object.getPrototypeOf(module) === Object.prototype) {
                ({module} = module)
            } else {
                console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
            }
        }

        const imports = __wbg_get_imports();

        __wbg_init_memory(imports);

        if (!(module instanceof WebAssembly.Module)) {
            module = new WebAssembly.Module(module);
        }

        const instance = new WebAssembly.Instance(module, imports);

        return __wbg_finalize_init(instance, module);
    }

    async function __wbg_init(module_or_path) {
        if (wasm !== undefined) return wasm;


        if (typeof module_or_path !== 'undefined') {
            if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
                ({module_or_path} = module_or_path)
            } else {
                console.warn('using deprecated parameters for the initialization function; pass a single object instead')
            }
        }

        if (typeof module_or_path === 'undefined' && typeof script_src !== 'undefined') {
            module_or_path = script_src.replace(/\.js$/, '_bg.wasm');
        }
        const imports = __wbg_get_imports();

        if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
            module_or_path = fetch(module_or_path);
        }

        __wbg_init_memory(imports);

        const { instance, module } = await __wbg_load(await module_or_path, imports);

        return __wbg_finalize_init(instance, module);
    }

    wasm_bindgen = Object.assign(__wbg_init, { initSync }, __exports);

})();
