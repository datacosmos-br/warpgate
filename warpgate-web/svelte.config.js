import sveltePreprocess from 'svelte-preprocess'

const production = process.env.NODE_ENV === 'production';

/** @type {import('@sveltejs/kit').Config} */
const config = {
    compilerOptions: {
        enableSourcemap: !production,
        dev: !production,
    },
    preprocess: sveltePreprocess({
        sourceMap: !production,
        scss: true,
        typescript: {
            tsconfigFile: './tsconfig.json',
        },
    }),
    vitePlugin: {
        prebundleSvelteLibraries: true,
    },
}

export default config

