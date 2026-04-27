import typescript from 'rollup-plugin-typescript2';
import resolve from '@rollup/plugin-node-resolve';

export default {
    input: 'src/index.ts', // Your entry file
    output: [
        {
            file: 'dist/bundle.cjs.js',
            format: 'cjs', // CommonJS format
            sourcemap: true,
        },
        {
            file: 'dist/bundle.esm.js',
            format: 'esm', // ES module format
            sourcemap: true,
        },
    ],
    plugins: [
        resolve(), // Resolves modules from node_modules
        typescript({
            tsconfig: './tsconfig.json',
        }),
    ],
};
