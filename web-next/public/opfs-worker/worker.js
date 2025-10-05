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

    let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

    cachedTextDecoder.decode();

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

    function addToExternrefTable0(obj) {
        const idx = wasm.__externref_table_alloc();
        wasm.__wbindgen_export_2.set(idx, obj);
        return idx;
    }

    function handleError(f, args) {
        try {
            return f.apply(this, args);
        } catch (e) {
            const idx = addToExternrefTable0(e);
            wasm.__wbindgen_exn_store(idx);
        }
    }

    function isLikeNone(x) {
        return x === undefined || x === null;
    }

    function _assertNum(n) {
        if (typeof(n) !== 'number') throw new Error(`expected a number argument, found ${typeof(n)}`);
    }

    let WASM_VECTOR_LEN = 0;

    const cachedTextEncoder = new TextEncoder();

    if (!('encodeInto' in cachedTextEncoder)) {
        cachedTextEncoder.encodeInto = function (arg, view) {
            const buf = cachedTextEncoder.encode(arg);
            view.set(buf);
            return {
                read: arg.length,
                written: buf.length
            };
        }
    }

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
            const ret = cachedTextEncoder.encodeInto(arg, view);
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

    const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(
    state => {
        wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b);
    }
    );

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
                    wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b); state.a = 0;
                    CLOSURE_DTORS.unregister(state);
                }
            }
        };
        real.original = state;
        CLOSURE_DTORS.register(real, state, state);
        return real;
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
     * @returns {Promise<boolean>}
     */
    __exports.is_compatible = function() {
        const ret = wasm.is_compatible();
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
     * Add device files to opfs
     * and return list of ResourceSelections
     * @param {Array<any>} files
     * @returns {Promise<Uint8Array>}
     */
    __exports.add_device_files = function(files) {
        const ret = wasm.add_device_files(files);
        return ret;
    };

    function passArrayJsValueToWasm0(array, malloc) {
        const ptr = malloc(array.length * 4, 4) >>> 0;
        for (let i = 0; i < array.length; i++) {
            const add = addToExternrefTable0(array[i]);
            getDataViewMemory0().setUint32(ptr + 4 * i, add, true);
        }
        WASM_VECTOR_LEN = array.length;
        return ptr;
    }
    /**
     * @param {string} path
     * @param {File[]} files
     * @returns {Promise<Uint8Array>}
     */
    __exports.add_device_folder = function(path, files) {
        const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(files, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.add_device_folder(ptr0, len0, ptr1, len1);
        return ret;
    };

    /**
     * @param {Uint8Array} path
     * @returns {Promise<File | undefined>}
     */
    __exports.get_device_file = function(path) {
        const ret = wasm.get_device_file(path);
        return ret;
    };

    /**
     * @param {Uint8Array} path
     * @returns {Promise<string | undefined>}
     */
    __exports.get_download_url = function(path) {
        const ret = wasm.get_download_url(path);
        return ret;
    };

    /**
     * Run CoreOperation and return the CoreOperationOutput
     * @param {Uint8Array} effect
     * @returns {Promise<Uint8Array>}
     */
    __exports.execute_operation = function(effect) {
        const ret = wasm.execute_operation(effect);
        return ret;
    };

    /**
     * Create file at path
     * @param {Uint8Array} file_path
     * @param {Uint8Array} data
     * @returns {Promise<void>}
     */
    __exports.create_file = function(file_path, data) {
        const ret = wasm.create_file(file_path, data);
        return ret;
    };

    /**
     * Run CoreOperation and call core to handle response
     * Return the next Operations that need to execute.
     * @param {number} request_id
     * @param {Uint8Array} effect
     * @returns {Promise<Uint8Array>}
     */
    __exports.execute = function(request_id, effect) {
        _assertNum(request_id);
        const ret = wasm.execute(request_id, effect);
        return ret;
    };

    function __wbg_adapter_6(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7909_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_9(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7713_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_14(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure276_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_17(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7710_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_20(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure277_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_23(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7855_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_26(arg0, arg1) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.wasm_bindgen__convert__closures_____invoke__ha59ecba20431b6e7(arg0, arg1);
    }

    function __wbg_adapter_31(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7711_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_34(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7712_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_37(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7856_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_40(arg0, arg1) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.wasm_bindgen__convert__closures_____invoke__hb5e047f6458a2444(arg0, arg1);
    }

    function __wbg_adapter_43(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure8471_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_48(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure8261_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_51(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure8148_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_54(arg0, arg1, arg2) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure7854_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_61(arg0, arg1) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.wasm_bindgen__convert__closures_____invoke__hff34c56f5e583ff1(arg0, arg1);
    }

    function __wbg_adapter_551(arg0, arg1, arg2, arg3) {
        _assertNum(arg0);
        _assertNum(arg1);
        wasm.closure8479_externref_shim(arg0, arg1, arg2, arg3);
    }

    const __wbindgen_enum_BinaryType = ["blob", "arraybuffer"];

    const __wbindgen_enum_IdbTransactionMode = ["readonly", "readwrite", "versionchange", "readwriteflush", "cleanup"];

    const __wbindgen_enum_ReadableStreamType = ["bytes"];

    const __wbindgen_enum_ReferrerPolicy = ["", "no-referrer", "no-referrer-when-downgrade", "origin", "origin-when-cross-origin", "unsafe-url", "same-origin", "strict-origin", "strict-origin-when-cross-origin"];

    const __wbindgen_enum_RequestCache = ["default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached"];

    const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];

    const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];

    const __wbindgen_enum_RequestRedirect = ["follow", "error", "manual"];

    const __wbindgen_enum_RtcDataChannelType = ["arraybuffer", "blob"];

    const __wbindgen_enum_RtcIceConnectionState = ["new", "checking", "connected", "completed", "failed", "disconnected", "closed"];

    const __wbindgen_enum_RtcIceGatheringState = ["new", "gathering", "complete"];

    const __wbindgen_enum_RtcSdpType = ["offer", "pranswer", "answer", "rollback"];

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
    if (Symbol.dispose) IntoUnderlyingByteSource.prototype[Symbol.dispose] = IntoUnderlyingByteSource.prototype.free;

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
    if (Symbol.dispose) IntoUnderlyingSink.prototype[Symbol.dispose] = IntoUnderlyingSink.prototype.free;

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
    if (Symbol.dispose) IntoUnderlyingSource.prototype[Symbol.dispose] = IntoUnderlyingSource.prototype.free;

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
        imports.wbg.__wbg_Error_e17e777aac105295 = function() { return logError(function (arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_Number_998bea33bd87c3e0 = function() { return logError(function (arg0) {
            const ret = Number(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_abort_67e1b49bf6614565 = function() { return logError(function (arg0) {
            arg0.abort();
        }, arguments) };
        imports.wbg.__wbg_abort_c1fed407cecc529e = function() { return handleError(function (arg0) {
            arg0.abort();
        }, arguments) };
        imports.wbg.__wbg_abort_d830bf2e9aa6ec5b = function() { return logError(function (arg0, arg1) {
            arg0.abort(arg1);
        }, arguments) };
        imports.wbg.__wbg_addIceCandidate_1fa425fe4613fd5c = function() { return logError(function (arg0, arg1) {
            const ret = arg0.addIceCandidate(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_add_1816c1d0ee7a76f5 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.add(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_add_9a58ce29756ac132 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.add(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_append_72a3c0addd2bce38 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_arrayBuffer_2c907ed8e8ef4e35 = function() { return logError(function (arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_arrayBuffer_9c99b8e2809e8cbb = function() { return handleError(function (arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_body_4851aa049324a851 = function() { return logError(function (arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_bound_99d0883606949696 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = IDBKeyRange.bound(arg0, arg1, arg2 !== 0, arg3 !== 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_buffer_8d40b1d762fb3c66 = function() { return logError(function (arg0) {
            const ret = arg0.buffer;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_bufferedAmount_ccbaed8112bc9680 = function() { return logError(function (arg0) {
            const ret = arg0.bufferedAmount;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_byobRequest_2c036bceca1e6037 = function() { return logError(function (arg0) {
            const ret = arg0.byobRequest;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_byteLength_331a6b5545834024 = function() { return logError(function (arg0) {
            const ret = arg0.byteLength;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_byteOffset_49a5b5608000358b = function() { return logError(function (arg0) {
            const ret = arg0.byteOffset;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_caches_12adc7af691f9083 = function() { return handleError(function (arg0) {
            const ret = arg0.caches;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_call_13410aac570ffff7 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_call_a5400b25a865cfd8 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_cancel_8bb5b8f4906b658a = function() { return logError(function (arg0) {
            const ret = arg0.cancel();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_candidate_26503c197acea430 = function() { return logError(function (arg0) {
            const ret = arg0.candidate;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_catch_c80ecae90cb8ed4e = function() { return logError(function (arg0, arg1) {
            const ret = arg0.catch(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_clearTimeout_5de27855b2967b4a = function() { return handleError(function (arg0, arg1) {
            arg0.clearTimeout(arg1);
        }, arguments) };
        imports.wbg.__wbg_clearTimeout_6222fede17abcb1a = function() { return logError(function (arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_clearTimeout_96804de0ab838f26 = function() { return logError(function (arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_close_6437264570d2d37f = function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_close_acc70a3bf97c46af = function() { return logError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_close_cccada6053ee3a65 = function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_close_d71a78219dc23e91 = function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments) };
        imports.wbg.__wbg_code_177e3bed72688e58 = function() { return logError(function (arg0) {
            const ret = arg0.code;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_code_89056d52bf1a8bb0 = function() { return logError(function (arg0) {
            const ret = arg0.code;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createAnswer_525c37456cf76314 = function() { return logError(function (arg0) {
            const ret = arg0.createAnswer();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createDataChannel_e332054c70ef4847 = function() { return logError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.createDataChannel(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createObjectStore_2112aa8eea18ea9d = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.createObjectStore(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createObjectURL_c80225986d2b928b = function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_createOffer_3df0685c2c8812c5 = function() { return logError(function (arg0) {
            const ret = arg0.createOffer();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createSyncAccessHandle_d06aab2e41a339b2 = function() { return logError(function (arg0) {
            const ret = arg0.createSyncAccessHandle();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_data_9ab529722bcc4e6c = function() { return logError(function (arg0) {
            const ret = arg0.data;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_delete_33e805b6d49fa644 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.delete(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_done_75ed0ee6dd243d9d = function() { return logError(function (arg0) {
            const ret = arg0.done;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_enqueue_452bc2343d1c2ff9 = function() { return handleError(function (arg0, arg1) {
            arg0.enqueue(arg1);
        }, arguments) };
        imports.wbg.__wbg_entries_1a3c3b9544532397 = function() { return logError(function (arg0) {
            const ret = arg0.entries();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_entries_2be2f15bd5554996 = function() { return logError(function (arg0) {
            const ret = Object.entries(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_error_118f1b830b6ccf22 = function() { return handleError(function (arg0) {
            const ret = arg0.error;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
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
        imports.wbg.__wbg_fetch_36d024dbd9192353 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = arg0.fetch(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_fetch_87aed7f306ec6d63 = function() { return logError(function (arg0, arg1) {
            const ret = arg0.fetch(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_fetch_f083e6da40cefe09 = function() { return logError(function (arg0, arg1) {
            const ret = fetch(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_fetch_f156d10be9a5c88a = function() { return logError(function (arg0) {
            const ret = fetch(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_flush_d2487a24f3bc3cf4 = function() { return handleError(function (arg0) {
            arg0.flush();
        }, arguments) };
        imports.wbg.__wbg_forwardcoreoperationoutput_484b25b51e680571 = function() { return logError(function (arg0, arg1) {
            const ret = core.forward_core_operation_output(arg0 >>> 0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getAllKeys_08c3ed5bd20b20ba = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getAllKeys(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getAllKeys_ae73b31344476374 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getAllKeys(arg1, arg2 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getAllKeys_b922249231cbd849 = function() { return handleError(function (arg0) {
            const ret = arg0.getAllKeys();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getAllResponseHeaders_a79f9b1f708e295b = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.getAllResponseHeaders();
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getAll_2783028eb1814671 = function() { return handleError(function (arg0) {
            const ret = arg0.getAll();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getAll_32ab1618e54bf9e5 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getAll(arg1, arg2 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getAll_ff5bd24743b1031a = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getAll(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getDirectoryHandle_0fb26677897f1e21 = function() { return logError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.getDirectoryHandle(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getDirectory_0483f3cec68d5bc4 = function() { return logError(function () {
            const ret = navigator.storage.getDirectory();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getFileHandle_9f23d09c2497fa5f = function() { return logError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.getFileHandle(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getFile_b5b8e018a785a851 = function() { return logError(function (arg0) {
            const ret = arg0.getFile();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getRandomValues_38a1ff1ea09f6cc7 = function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments) };
        imports.wbg.__wbg_getReader_48e00749fe3f6089 = function() { return handleError(function (arg0) {
            const ret = arg0.getReader();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getSize_56a06761973a6cd7 = function() { return handleError(function (arg0) {
            const ret = arg0.getSize();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getTime_6bb3f64e0f18f817 = function() { return logError(function (arg0) {
            const ret = arg0.getTime();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getTimezoneOffset_1e3ddc1382e7c8b0 = function() { return logError(function (arg0) {
            const ret = arg0.getTimezoneOffset();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_get_0da715ceaecea5c8 = function() { return logError(function (arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        }, arguments) };
        imports.wbg.__wbg_get_1b2c33a63c4be73f = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.get(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_get_458e874b43b18b25 = function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getdone_f026246f6bbe58d3 = function() { return logError(function (arg0) {
            const ret = arg0.done;
            if (!isLikeNone(ret)) {
                _assertBoolean(ret);
            }
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        }, arguments) };
        imports.wbg.__wbg_getvalue_31e5a08f61e5aa42 = function() { return logError(function (arg0) {
            const ret = arg0.value;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getwithrefkey_1dc361bd10053bfe = function() { return logError(function (arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        }, arguments) };
        imports.wbg.__wbg_has_b89e451f638123e3 = function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.has(arg0, arg1);
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_headers_29fec3c72865cd75 = function() { return logError(function (arg0) {
            const ret = arg0.headers;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_href_65a798194bf5ead5 = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_iceConnectionState_17555f1b8f904bb4 = function() { return logError(function (arg0) {
            const ret = arg0.iceConnectionState;
            return (__wbindgen_enum_RtcIceConnectionState.indexOf(ret) + 1 || 8) - 1;
        }, arguments) };
        imports.wbg.__wbg_iceGatheringState_4c86463f33bdff67 = function() { return logError(function (arg0) {
            const ret = arg0.iceGatheringState;
            return (__wbindgen_enum_RtcIceGatheringState.indexOf(ret) + 1 || 4) - 1;
        }, arguments) };
        imports.wbg.__wbg_instanceof_ArrayBuffer_67f3012529f6a2dd = function() { return logError(function (arg0) {
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
        imports.wbg.__wbg_instanceof_Blob_3db67efd3f1b960f = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof Blob;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_FileSystemFileHandle_10a0ba9b32926641 = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemFileHandle;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_File_9b203ca2ca274154 = function() { return logError(function (arg0) {
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
        imports.wbg.__wbg_instanceof_IdbDatabase_6e6efef94c4a355d = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof IDBDatabase;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_IdbFactory_653c0aade11afa7c = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof IDBFactory;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_IdbOpenDbRequest_2be27facb05c6739 = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof IDBOpenDBRequest;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_IdbRequest_a4a68ff63181a915 = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof IDBRequest;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_Map_ebb01a5b6b5ffd0b = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof Map;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_Response_50fde2cd696850bf = function() { return logError(function (arg0) {
            let result;
            try {
                result = arg0 instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_instanceof_Uint8Array_9a8378d955933db7 = function() { return logError(function (arg0) {
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
        imports.wbg.__wbg_instanceof_Window_12d20d558ef92592 = function() { return logError(function (arg0) {
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
        imports.wbg.__wbg_isArray_030cce220591fb41 = function() { return logError(function (arg0) {
            const ret = Array.isArray(arg0);
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_isSafeInteger_1c0d1af5542e102a = function() { return logError(function (arg0) {
            const ret = Number.isSafeInteger(arg0);
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_iterator_f370b34483c71a1c = function() { return logError(function () {
            const ret = Symbol.iterator;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_lastModified_bba7ecb829c9236e = function() { return logError(function (arg0) {
            const ret = arg0.lastModified;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_length_186546c51cd61acd = function() { return logError(function (arg0) {
            const ret = arg0.length;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_length_6bb7e81f9d7713e4 = function() { return logError(function (arg0) {
            const ret = arg0.length;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_loaded_f56548c8804db859 = function() { return logError(function (arg0) {
            const ret = arg0.loaded;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_localDescription_cf5a4b638234c97a = function() { return logError(function (arg0) {
            const ret = arg0.localDescription;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_location_92d89c32ae076cab = function() { return logError(function (arg0) {
            const ret = arg0.location;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_log_6c7b5f4f00b8ce3f = function() { return logError(function (arg0) {
            console.log(arg0);
        }, arguments) };
        imports.wbg.__wbg_lowerBound_5a50c0a9f6e7db91 = function() { return handleError(function (arg0, arg1) {
            const ret = IDBKeyRange.lowerBound(arg0, arg1 !== 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_message_5481231e71ccaf7b = function() { return logError(function (arg0, arg1) {
            const ret = arg1.message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_name_42b465b8043f111e = function() { return logError(function (arg0, arg1) {
            const ret = arg1.name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_navigator_65d5ad763926b868 = function() { return logError(function (arg0) {
            const ret = arg0.navigator;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new0_b0a0a38c201e6df5 = function() { return logError(function () {
            const ret = new Date();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_19c25a3f2fa63a02 = function() { return logError(function () {
            const ret = new Object();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_1f3a344cf3123716 = function() { return logError(function () {
            const ret = new Array();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_2e3c58a15f39f5f9 = function() { return logError(function (arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return __wbg_adapter_551(a, state0.b, arg0, arg1);
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
        imports.wbg.__wbg_new_2ff1f68f3676ea53 = function() { return logError(function () {
            const ret = new Map();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_5a2ae4557f92b50e = function() { return logError(function (arg0) {
            const ret = new Date(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_638ebfaedbf32a5e = function() { return logError(function (arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_66b9434b4e59b63e = function() { return handleError(function () {
            const ret = new AbortController();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_8a6f238a6ece86ea = function() { return logError(function () {
            const ret = new Error();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_9d476835fd376de6 = function() { return handleError(function (arg0, arg1) {
            const ret = new Worker(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_bd4cca60314f67a3 = function() { return handleError(function () {
            const ret = new FileReader();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_bfeae14e81f41b77 = function() { return handleError(function () {
            const ret = new XMLHttpRequest();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_da9dc54c5db29dfa = function() { return logError(function (arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_e213f63d18b0de01 = function() { return handleError(function (arg0, arg1) {
            const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_f6e53210afea8e45 = function() { return handleError(function () {
            const ret = new Headers();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newfromslice_074c56947bd43469 = function() { return logError(function (arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newnoargs_254190557c45b4ec = function() { return logError(function (arg0, arg1) {
            const ret = new Function(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithbase_96f007ba18c568ff = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = new URL(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithbyteoffsetandlength_e8f53910b4d42b45 = function() { return logError(function (arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithconfiguration_f181e8239305687f = function() { return handleError(function (arg0) {
            const ret = new RTCPeerConnection(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithlength_a167dcc7aaa3ba77 = function() { return logError(function (arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithstrandinit_b5d168a29a3fd85f = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithstrsequence_f7e2d4848dd49d98 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new WebSocket(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithstrsequenceandoptions_5b257525e688af7d = function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_next_1142e1658f75ec63 = function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_next_5b3530e612fde77d = function() { return logError(function (arg0) {
            const ret = arg0.next;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_next_692e82279131b03c = function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_now_2c95c9de01293173 = function() { return logError(function (arg0) {
            const ret = arg0.now();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_now_98430d19d580dbab = function() { return handleError(function () {
            const ret = Date.now();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_objectStore_b2a5b80b2e5c5f8b = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.objectStore(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_open_191703f9f86f45a7 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4), arg5 !== 0);
        }, arguments) };
        imports.wbg.__wbg_open_7281831ed8ff7bd2 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.open(getStringFromWasm0(arg1, arg2), arg3 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_open_f25e984ff3e90fbe = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.open(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_parse_442f5ba02e5eaf8b = function() { return handleError(function (arg0, arg1) {
            const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_performance_7a3ffd0b17f663ad = function() { return logError(function (arg0) {
            const ret = arg0.performance;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_postMessage_38909232d65f5870 = function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments) };
        imports.wbg.__wbg_postMessage_50e57097ede408b9 = function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments) };
        imports.wbg.__wbg_prototypesetcall_3d4a26c1ed734349 = function() { return logError(function (arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        }, arguments) };
        imports.wbg.__wbg_push_330b2eb93e4e1212 = function() { return logError(function (arg0, arg1) {
            const ret = arg0.push(arg1);
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_put_cdfadd5d7f714201 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.put(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_put_f777be76774b073e = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.put(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_queueMicrotask_25d0739ac89e8c88 = function() { return logError(function (arg0) {
            queueMicrotask(arg0);
        }, arguments) };
        imports.wbg.__wbg_queueMicrotask_4488407636f5bf24 = function() { return logError(function (arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_readAsArrayBuffer_a9f65e0c524f16a0 = function() { return handleError(function (arg0, arg1) {
            arg0.readAsArrayBuffer(arg1);
        }, arguments) };
        imports.wbg.__wbg_read_bc925c758aa4d897 = function() { return logError(function (arg0) {
            const ret = arg0.read();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_readyState_b0d20ca4531d3797 = function() { return logError(function (arg0) {
            const ret = arg0.readyState;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_reason_97efd955be6394bd = function() { return logError(function (arg0, arg1) {
            const ret = arg1.reason;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_releaseLock_ff29b586502a8221 = function() { return logError(function (arg0) {
            arg0.releaseLock();
        }, arguments) };
        imports.wbg.__wbg_replace_70b0a5b9f274520a = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.replace(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_resolve_4055c623acdd6a1b = function() { return logError(function (arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_respond_6c2c4e20ef85138e = function() { return handleError(function (arg0, arg1) {
            arg0.respond(arg1 >>> 0);
        }, arguments) };
        imports.wbg.__wbg_response_ac4b1ccdd0140db5 = function() { return handleError(function (arg0) {
            const ret = arg0.response;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_result_825a6aeeb31189d2 = function() { return handleError(function (arg0) {
            const ret = arg0.result;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_result_f3b8657ddb4b49e7 = function() { return handleError(function (arg0) {
            const ret = arg0.result;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_sdp_36e268d45cbea0d8 = function() { return logError(function (arg0, arg1) {
            const ret = arg1.sdp;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_send_51a657a89ea3285a = function() { return handleError(function (arg0, arg1) {
            arg0.send(arg1);
        }, arguments) };
        imports.wbg.__wbg_send_61e2f6d0f5df06f3 = function() { return handleError(function (arg0) {
            arg0.send();
        }, arguments) };
        imports.wbg.__wbg_send_8c9f6a77391eabf6 = function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(getArrayU8FromWasm0(arg1, arg2));
        }, arguments) };
        imports.wbg.__wbg_send_aa9cb445685f0fd0 = function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(getArrayU8FromWasm0(arg1, arg2));
        }, arguments) };
        imports.wbg.__wbg_send_b2f8b041beebf459 = function() { return handleError(function (arg0, arg1) {
            arg0.send(arg1);
        }, arguments) };
        imports.wbg.__wbg_send_bdda9fac7465e036 = function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(getStringFromWasm0(arg1, arg2));
        }, arguments) };
        imports.wbg.__wbg_send_dc4585e7805d770e = function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2));
        }, arguments) };
        imports.wbg.__wbg_send_f94e0d26d4565303 = function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(arg1 === 0 ? undefined : getArrayU8FromWasm0(arg1, arg2));
        }, arguments) };
        imports.wbg.__wbg_setLocalDescription_884f2b8b04726c38 = function() { return logError(function (arg0, arg1) {
            const ret = arg0.setLocalDescription(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_setRemoteDescription_920f9a5013925c10 = function() { return logError(function (arg0, arg1) {
            const ret = arg0.setRemoteDescription(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_setRequestHeader_0b65c1fd8d5a4424 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setRequestHeader(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setTimeout_2b339866a2aa3789 = function() { return logError(function (arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_setTimeout_eefe7f4c234b0c6b = function() { return handleError(function (arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_setTimeout_fe5a06d54df0b75c = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_set_1353b2a5e96bc48c = function() { return logError(function (arg0, arg1, arg2) {
            arg0.set(getArrayU8FromWasm0(arg1, arg2));
        }, arguments) };
        imports.wbg.__wbg_set_1c17f9738fac2718 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_set_3f1d0b984ed272ed = function() { return logError(function (arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        }, arguments) };
        imports.wbg.__wbg_set_90f6c0f7bd8c0415 = function() { return logError(function (arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        }, arguments) };
        imports.wbg.__wbg_set_b7f1cf4fae26fe2a = function() { return logError(function (arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_setat_f8fc70f546036b10 = function() { return logError(function (arg0, arg1) {
            arg0.at = arg1;
        }, arguments) };
        imports.wbg.__wbg_setautoincrement_50a19db9199c2ec6 = function() { return logError(function (arg0, arg1) {
            arg0.autoIncrement = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setbinaryType_37f3cd35d7775a47 = function() { return logError(function (arg0, arg1) {
            arg0.binaryType = __wbindgen_enum_BinaryType[arg1];
        }, arguments) };
        imports.wbg.__wbg_setbinaryType_bc75414bc6bef66a = function() { return logError(function (arg0, arg1) {
            arg0.binaryType = __wbindgen_enum_RtcDataChannelType[arg1];
        }, arguments) };
        imports.wbg.__wbg_setbody_c8460bdf44147df8 = function() { return logError(function (arg0, arg1) {
            arg0.body = arg1;
        }, arguments) };
        imports.wbg.__wbg_setcache_90ca4ad8a8ad40d3 = function() { return logError(function (arg0, arg1) {
            arg0.cache = __wbindgen_enum_RequestCache[arg1];
        }, arguments) };
        imports.wbg.__wbg_setcreate_1eb73f4ea713c1ad = function() { return logError(function (arg0, arg1) {
            arg0.create = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setcreate_2d32aa4bbcd1d7af = function() { return logError(function (arg0, arg1) {
            arg0.create = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setcredentials_9cd60d632c9d5dfc = function() { return logError(function (arg0, arg1) {
            arg0.credentials = __wbindgen_enum_RequestCredentials[arg1];
        }, arguments) };
        imports.wbg.__wbg_setheaders_0052283e2f3503d1 = function() { return logError(function (arg0, arg1) {
            arg0.headers = arg1;
        }, arguments) };
        imports.wbg.__wbg_seticeservers_e21ca43974caf34d = function() { return logError(function (arg0, arg1) {
            arg0.iceServers = arg1;
        }, arguments) };
        imports.wbg.__wbg_setid_09a92dd26df112d3 = function() { return logError(function (arg0, arg1) {
            arg0.id = arg1;
        }, arguments) };
        imports.wbg.__wbg_setintegrity_de8bf847597602b5 = function() { return logError(function (arg0, arg1, arg2) {
            arg0.integrity = getStringFromWasm0(arg1, arg2);
        }, arguments) };
        imports.wbg.__wbg_setkeypath_3a5536ae3a5f612c = function() { return logError(function (arg0, arg1) {
            arg0.keyPath = arg1;
        }, arguments) };
        imports.wbg.__wbg_setmaxretransmits_8b4c70304f525d38 = function() { return logError(function (arg0, arg1) {
            arg0.maxRetransmits = arg1;
        }, arguments) };
        imports.wbg.__wbg_setmethod_9b504d5b855b329c = function() { return logError(function (arg0, arg1, arg2) {
            arg0.method = getStringFromWasm0(arg1, arg2);
        }, arguments) };
        imports.wbg.__wbg_setmode_a23e1a2ad8b512f8 = function() { return logError(function (arg0, arg1) {
            arg0.mode = __wbindgen_enum_RequestMode[arg1];
        }, arguments) };
        imports.wbg.__wbg_setnegotiated_8d88946a5a8aac5a = function() { return logError(function (arg0, arg1) {
            arg0.negotiated = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setonabort_1c7c4fbf1bae76fb = function() { return logError(function (arg0, arg1) {
            arg0.onabort = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonclose_159c0332c2d91b09 = function() { return logError(function (arg0, arg1) {
            arg0.onclose = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonclose_ef981e4d15715d71 = function() { return logError(function (arg0, arg1) {
            arg0.onclose = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonerror_088538f1d93e5bdb = function() { return logError(function (arg0, arg1) {
            arg0.onerror = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonerror_5d9bff045f909e89 = function() { return logError(function (arg0, arg1) {
            arg0.onerror = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonerror_8e948291b431c538 = function() { return logError(function (arg0, arg1) {
            arg0.onerror = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonerror_bcdbd7f3921ffb1f = function() { return logError(function (arg0, arg1) {
            arg0.onerror = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonicecandidate_9dd5524245a7c33e = function() { return logError(function (arg0, arg1) {
            arg0.onicecandidate = arg1;
        }, arguments) };
        imports.wbg.__wbg_setoniceconnectionstatechange_1f7f884c6bedf0a9 = function() { return logError(function (arg0, arg1) {
            arg0.oniceconnectionstatechange = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonicegatheringstatechange_3ae9f3bfdc85a459 = function() { return logError(function (arg0, arg1) {
            arg0.onicegatheringstatechange = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonload_c8268257798c74fb = function() { return logError(function (arg0, arg1) {
            arg0.onload = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonloadend_3599b3737c0c5708 = function() { return logError(function (arg0, arg1) {
            arg0.onloadend = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonmessage_041fb8f4204d276c = function() { return logError(function (arg0, arg1) {
            arg0.onmessage = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonmessage_5e486f326638a9da = function() { return logError(function (arg0, arg1) {
            arg0.onmessage = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonmessage_c943f7891405ab22 = function() { return logError(function (arg0, arg1) {
            arg0.onmessage = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonmessage_f05c58861e16d834 = function() { return logError(function (arg0, arg1) {
            arg0.onmessage = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonopen_3e43af381c2901f8 = function() { return logError(function (arg0, arg1) {
            arg0.onopen = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonopen_ec2a5a023d07dd05 = function() { return logError(function (arg0, arg1) {
            arg0.onopen = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonprogress_f3da0e33bd7c51a9 = function() { return logError(function (arg0, arg1) {
            arg0.onprogress = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonsuccess_ffb2ddb27ce681d8 = function() { return logError(function (arg0, arg1) {
            arg0.onsuccess = arg1;
        }, arguments) };
        imports.wbg.__wbg_setonupgradeneeded_4e32d1c6a08c4257 = function() { return logError(function (arg0, arg1) {
            arg0.onupgradeneeded = arg1;
        }, arguments) };
        imports.wbg.__wbg_setordered_819b64cfa203e299 = function() { return logError(function (arg0, arg1) {
            arg0.ordered = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setredirect_9542307f3ab946a9 = function() { return logError(function (arg0, arg1) {
            arg0.redirect = __wbindgen_enum_RequestRedirect[arg1];
        }, arguments) };
        imports.wbg.__wbg_setreferrer_c8dd38f95f31e178 = function() { return logError(function (arg0, arg1, arg2) {
            arg0.referrer = getStringFromWasm0(arg1, arg2);
        }, arguments) };
        imports.wbg.__wbg_setreferrerpolicy_164abad8ed6e3886 = function() { return logError(function (arg0, arg1) {
            arg0.referrerPolicy = __wbindgen_enum_ReferrerPolicy[arg1];
        }, arguments) };
        imports.wbg.__wbg_setsdp_ce437994391f1156 = function() { return logError(function (arg0, arg1, arg2) {
            arg0.sdp = getStringFromWasm0(arg1, arg2);
        }, arguments) };
        imports.wbg.__wbg_setsignal_8c45ad1247a74809 = function() { return logError(function (arg0, arg1) {
            arg0.signal = arg1;
        }, arguments) };
        imports.wbg.__wbg_settype_298968e371b58a33 = function() { return logError(function (arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        }, arguments) };
        imports.wbg.__wbg_settype_87f8a4c04fc0733c = function() { return logError(function (arg0, arg1) {
            arg0.type = __wbindgen_enum_RtcSdpType[arg1];
        }, arguments) };
        imports.wbg.__wbg_signal_da4d466ce86118b5 = function() { return logError(function (arg0) {
            const ret = arg0.signal;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_size_8f84e7768fba0589 = function() { return logError(function (arg0) {
            const ret = arg0.size;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_slice_2208fb0cdd166020 = function() { return handleError(function (arg0) {
            const ret = arg0.slice();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_slice_224856d46230c13c = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.slice(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_slice_36c55c8c1a260c46 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.slice(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_stack_0ed75d68575b0f3c = function() { return logError(function (arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_GLOBAL_8921f820c2ce3f12 = function() { return logError(function () {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_GLOBAL_THIS_f0a4409105898184 = function() { return logError(function () {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_SELF_995b214ae681ff99 = function() { return logError(function () {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_static_accessor_WINDOW_cde3890479c675ea = function() { return logError(function () {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_status_3fea3036088621d6 = function() { return logError(function (arg0) {
            const ret = arg0.status;
            _assertNum(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_storage_240bcb14726ee227 = function() { return logError(function (arg0) {
            const ret = arg0.storage;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_stringify_b98c93d0a190446a = function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_target_f2c963b447be6283 = function() { return logError(function (arg0) {
            const ret = arg0.target;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_then_b33a773d723afa3e = function() { return logError(function (arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_then_e22500defe16819f = function() { return logError(function (arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_toISOString_f5382b37d44a0082 = function() { return logError(function (arg0) {
            const ret = arg0.toISOString();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_toJSON_40172262eceef523 = function() { return logError(function (arg0) {
            const ret = arg0.toJSON();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_toString_78df35411a4fd40c = function() { return logError(function (arg0) {
            const ret = arg0.toString();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_transaction_e94a54f60797ce82 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.transaction(arg1, __wbindgen_enum_IdbTransactionMode[arg2]);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_type_286052cf9318fb63 = function() { return logError(function (arg0, arg1) {
            const ret = arg1.type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_updateappevent_b051ac6eba9da569 = function() { return logError(function (arg0) {
            const ret = core.update_app_event(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_upload_f07a1ef5ae102632 = function() { return handleError(function (arg0) {
            const ret = arg0.upload;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_upperBound_884e6dbf6030d98b = function() { return handleError(function (arg0, arg1) {
            const ret = IDBKeyRange.upperBound(arg0, arg1 !== 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_url_18b0690200329f32 = function() { return logError(function (arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_url_e5720dfacf77b05e = function() { return logError(function (arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_value_dd9372230531eade = function() { return logError(function (arg0) {
            const ret = arg0.value;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_view_91cc97d57ab30530 = function() { return logError(function (arg0) {
            const ret = arg0.view;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_wasClean_ffb515fbcbcbdd3d = function() { return logError(function (arg0) {
            const ret = arg0.wasClean;
            _assertBoolean(ret);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_wbindgenbigintgetasi64_ac743ece6ab9bba1 = function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            if (!isLikeNone(ret)) {
                _assertBigInt(ret);
            }
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        };
        imports.wbg.__wbg_wbindgenbooleanget_3fe6f642c7d97746 = function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            if (!isLikeNone(ret)) {
                _assertBoolean(ret);
            }
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        };
        imports.wbg.__wbg_wbindgencbdrop_eb10308566512b88 = function(arg0) {
            const obj = arg0.original;
            if (obj.cnt-- == 1) {
                obj.a = 0;
                return true;
            }
            const ret = false;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgendebugstring_99ef257a3ddda34d = function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_wbindgenin_d7a1ee10933d2d55 = function(arg0, arg1) {
            const ret = arg0 in arg1;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenisbigint_ecb90cc08a5a9154 = function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenisfunction_8cee7dce3725ae74 = function(arg0) {
            const ret = typeof(arg0) === 'function';
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenisnull_f3037694abe4d97a = function(arg0) {
            const ret = arg0 === null;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenisobject_307a53c6bd97fbf8 = function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenisstring_d4fa939789f003b0 = function(arg0) {
            const ret = typeof(arg0) === 'string';
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenisundefined_c4b71d073b92f3c5 = function(arg0) {
            const ret = arg0 === undefined;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenjsvaleq_e6f2ad59ccae1b58 = function(arg0, arg1) {
            const ret = arg0 === arg1;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgenjsvallooseeq_9bec8c9be826bed1 = function(arg0, arg1) {
            const ret = arg0 == arg1;
            _assertBoolean(ret);
            return ret;
        };
        imports.wbg.__wbg_wbindgennumberget_f74b4c7525ac05cb = function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            if (!isLikeNone(ret)) {
                _assertNum(ret);
            }
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        };
        imports.wbg.__wbg_wbindgenstringget_0f16a6ddddef376f = function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_wbindgenthrow_451ec1a8469d7eb6 = function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_write_800b952042eda528 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.write(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_write_c0e234eeb0039d0d = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.write(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_03e4ac2d3b915214 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7907, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 7909, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7907, __wbg_adapter_6);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_1f0e0ea0793b1f11 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 8150, function: Function { arguments: [NamedExternref("ProgressEvent")], shim_idx: 8148, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 8150, __wbg_adapter_51);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_2241b6af4c4b2941 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_2c2a2132f5871a60 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7863, function: Function { arguments: [NamedExternref("RTCPeerConnectionIceEvent")], shim_idx: 7856, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7863, __wbg_adapter_37);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_2de4a289ecafc347 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 318, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 276, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, 318, __wbg_adapter_14);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_2f748feaed5039e4 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7694, function: Function { arguments: [NamedExternref("ErrorEvent")], shim_idx: 7710, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7694, __wbg_adapter_17);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_4625c577ab2ec9ee = function() { return logError(function (arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_6a7174cff28fc48d = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7862, function: Function { arguments: [NamedExternref("Event")], shim_idx: 7854, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7862, __wbg_adapter_54);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_734923ebffbb87e5 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 8259, function: Function { arguments: [NamedExternref("Event")], shim_idx: 8261, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 8259, __wbg_adapter_48);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_76c5be8a73eef865 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7696, function: Function { arguments: [Externref], shim_idx: 7711, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7696, __wbg_adapter_31);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_7ae06049086d89b9 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 8469, function: Function { arguments: [Externref], shim_idx: 8471, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 8469, __wbg_adapter_43);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_7e83186c2580f4fc = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7697, function: Function { arguments: [NamedExternref("ProgressEvent")], shim_idx: 7713, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7697, __wbg_adapter_9);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_86d9a770984081ea = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 8315, function: Function { arguments: [], shim_idx: 8317, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 8315, __wbg_adapter_40);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_9ae0607507abb057 = function() { return logError(function (arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_bde382b4249e9445 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 8412, function: Function { arguments: [], shim_idx: 8414, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 8412, __wbg_adapter_61);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_cb9088102bce6b30 = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_d4a5de91d76c656f = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 317, function: Function { arguments: [NamedExternref("IDBVersionChangeEvent")], shim_idx: 277, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 317, __wbg_adapter_20);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_d6cd19b81560fd6e = function() { return logError(function (arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_d8ecf399d91ec72e = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7695, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 7712, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7695, __wbg_adapter_34);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_d980f0682c086a8e = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7674, function: Function { arguments: [], shim_idx: 7676, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7674, __wbg_adapter_26);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_cast_fdfb020f24308beb = function() { return logError(function (arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 7864, function: Function { arguments: [NamedExternref("CloseEvent")], shim_idx: 7855, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, 7864, __wbg_adapter_23);
            return ret;
        }, arguments) };
        imports.wbg.__wbindgen_init_externref_table = function() {
            const table = wasm.__wbindgen_export_2;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
            ;
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
