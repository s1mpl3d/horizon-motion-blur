# Horizon Motion Blur

Mod nativo (`.hnmod`) de motion blur para o [Horizon](https://github.com/horizon-rust/horizon), o Minecraft reescrito em Rust.

O mundo é borrado ao longo do movimento da câmera, como o obturador de uma câmera. A mão, o HUD e as telas são desenhados depois do blur, então continuam nítidos.

## Instalação

1. Baixe `motion_blur.hnmod`.
2. Coloque em `%APPDATA%\horizon\horizon-native-mods\`.
3. Abra o launcher do Horizon e recompile os mods nativos.
4. No jogo, ative em **Options > Mods > Motion Blur**.

## Configurações

| Opção | Valores | Padrão |
| --- | --- | --- |
| Strength | 25%, 50%, 75%, 100%, 150%, 200% | 75% |
| Quality | 8, 12, 16, 24 amostras | 12 |
| Same at any FPS | ON / OFF | ON |

**Same at any FPS** mantém o blur com o tamanho que teria a 60 FPS. Desligado, o blur diminui quanto maior o FPS, como numa câmera real.

As configurações ficam em `%APPDATA%\horizon\horizon-mods\.horizon-mod-data\motion_blur\`.

## Como funciona

O mod usa o hook `native_world_drawn`, que roda depois que o mundo é desenhado e antes da mão e do HUD. Para cada pixel, ele:

1. lê a profundidade do mundo (pedida via `native_render_needs`);
2. reconstrói a posição do ponto relativa à câmera;
3. projeta essa posição com as matrizes do frame anterior;
4. tira várias amostras da imagem ao longo da diferença entre as duas posições na tela.

É a mesma ideia de reprojeção do [natural-motionblur](https://github.com/ItsPasi/natural-motionblur-fabric) e do [motionblur](https://github.com/Noryea/motionblur-fabric). O blur vem só do movimento da câmera: entidades que se movem com a câmera parada não são borradas. Em teleportes (mais de 16 blocos num frame) o blur é pulado.

## Compatibilidade

- Precisa de `"hooks": 58` (os hooks de render), então compila em qualquer Horizon com essa versão de hooks ou mais nova.
- Testado no commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (Minecraft 26.3): o `horizon-mixins` aceita o mod e o `cargo check` do cliente passa sem erros.
- No OpenGL, se a imagem do frame não puder ser copiada (`COPY_SRC`), o mod não faz nada.

## Código

- `motion_blur/horizon-native.json`: manifesto do mod.
- `motion_blur/mixins/motion_blur/horizon-mixin.toml`: módulos e injeções.
- `motion_blur/mixins/motion_blur/runtime.rs`: configurações, shader WGSL e o passe de blur.
- Os outros `.rs` são os helpers de cada hook.

Para gerar o `.hnmod`, compacte em zip o conteúdo da pasta `motion_blur/` (com `horizon-native.json` na raiz do zip).
