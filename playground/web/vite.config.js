import { defineConfig } from 'vite';
import { viteStaticCopy } from 'vite-plugin-static-copy';
import { execSync } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Plugin to build tutorials before Vite build
function buildTutorialsPlugin() {
  return {
    name: 'build-tutorials',
    buildStart() {
      console.log('🖖 Building tutorials from markdown...');
      try {
        execSync('node scripts/build-tutorials.js', {
          cwd: __dirname,
          stdio: 'inherit'
        });
      } catch (error) {
        console.error('Failed to build tutorials:', error);
        throw error;
      }
    }
  };
}

export default defineConfig({
  plugins: [
    buildTutorialsPlugin(),
    viteStaticCopy({
      targets: [
        {
          src: '../../vscode/language-configuration.json',
          dest: '.'
        }
      ]
    })
  ],
  build: {
    outDir: 'dist',
    // Don't minify for easier debugging (can enable later)
    minify: false,
    // Keep directory structure
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, 'index.html'),
        tutorial: path.resolve(__dirname, 'tutorial.html'),
        embed: path.resolve(__dirname, 'embed.html'),
      }
    }
  },
  server: {
    port: 5173,
    open: true
  }
});
