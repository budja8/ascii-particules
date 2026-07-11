# Rust Fun Apps

Este proyecto contiene dos aplicaciones divertidas, interactivas y altamente eficientes construidas en **Rust**:

1. **Generador de Arte ASCII (`ascii`)**: Un convertidor CLI que transforma imágenes en arte ASCII en tiempo real en la terminal, con colores verdaderos (Truecolor de 24 bits) y animación de construcción línea por línea.
2. **Simulador de Gravedad de Partículas (`particles`)**: Un simulador de físicas en 2D interactivo con gráficos acelerados por hardware (OpenGL), soporte para múltiples modos visuales (como redes fluidas, espectrogramas circulares y osciloscopios) y reactividad en tiempo real al audio del sistema.

---

## Requisitos Previos

- Tener instalado [Rust y Cargo](https://rustup.rs/) (versión 1.70 o superior recomendada).
- Para el simulador de partículas reactivo al audio, asegúrate de tener una tarjeta de sonido activa en Windows (el programa captura automáticamente la salida predeterminada de tus altavoces o auriculares).

---

## Instalación y Compilación

Para compilar ambas aplicaciones en su modo óptimo optimizado (modo **Release**):

1. Abre tu terminal en la carpeta del proyecto.
2. Compila el proyecto con Cargo:
   ```bash
   cargo build --release
   ```
Esto generará los binarios ejecutables independientes en la carpeta `target/release/`:
- `target/release/ascii.exe` (Generador ASCII, tamaño: ~2.4 MB)
- `target/release/particles.exe` (Simulador de físicas, tamaño: ~820 KB)

---

## 1. Generador de Arte ASCII (`ascii`)

Convierte cualquier imagen (PNG, JPG, WEBP, etc.) en arte ASCII de colores directly en tu terminal.

### Comandos de Uso

- **Ejecución básica (Con color y animación por defecto):**
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png"
  ```
  *(O usando el ejecutable directo: `.\target\release\ascii.exe "ruta/a/tu/imagen.png"`)*

- **Ejecución en blanco y negro (Monocromo):**
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png" --mono
  ```

- **Ajustar el ancho del renderizado (Columnas de texto, por defecto 100):**
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png" --width 80
  ```

- **Ajustar la velocidad de la animación (Retardo en milisegundos entre líneas, por defecto 30ms):**
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png" --delay 15
  ```
  *Nota: Pasa `--delay 0` si deseas imprimir la imagen de forma instantánea sin animación.*

- **Invertir la rampa de brillo (Ideal para terminales con fondos claros):**
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png" --invert
  ```

- **Ver menú de ayuda completo:**
  ```bash
  cargo run --release --bin ascii -- --help
  ```

- **Animar la imagen con rotación de 360° (modo movimiento):**
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png" --rotate
  ```
  La imagen rota en el mismo lugar de la terminal en un loop continuo hasta que presiones
  `Ctrl+C`. Se puede ajustar la suavidad y velocidad de la animación:
  ```bash
  cargo run --release --bin ascii -- "ruta/a/tu/imagen.png" --rotate --frames 36 --frame-delay 80
  ```
  - `--frames`: cantidad de pasos para completar una vuelta de 360° (por defecto 60, más
    pasos = rotación más suave).
  - `--frame-delay`: milisegundos entre cada frame (por defecto 50ms).

  *Nota: esta es una primera versión sencilla. En imágenes no cuadradas, las esquinas
  pueden recortarse levemente en ángulos que no son múltiplos de 90°, y la animación no
  reacciona si cambias el tamaño de la terminal mientras corre.*

- **Reproducir una secuencia de imágenes en orden (modo GIF-como-ASCII):**
  ```bash
  cargo run --release --bin ascii -- --six
  ```
  Carga todas las imágenes de la carpeta `assets/six` (ordenadas por nombre de archivo,
  por eso usan un numerado con ceros a la izquierda como `frame-001.jpg`), las precalcula
  como ASCII y las reproduce en orden en el mismo lugar de la terminal, en loop continuo
  como un GIF hasta que presiones `Ctrl+C`. También acepta una carpeta distinta:
  ```bash
  cargo run --release --bin ascii -- --six "ruta/a/otra/carpeta"
  ```
  Reutiliza `--width`/`--height`/`--mono`/`--invert` igual que los demás modos. El delay
  entre frames por defecto es de **1500ms (1.5s)** para este modo (a diferencia de
  `--rotate`, que usa 50ms por defecto); se puede ajustar con `--frame-delay`:
  ```bash
  cargo run --release --bin ascii -- --six --frame-delay 500
  ```
  *Recomendado ejecutar el binario compilado en modo `--release` (o
  `target/release/ascii.exe` directo): decodificar muchas imágenes en modo debug es
  notablemente más lento.*

---

## 2. Simulador de Gravedad de Partículas (`particles`)

Una ventana interactiva en 2D que renderiza miles de elementos a 60 FPS. Además, captura el audio del sistema y lo separa en tiempo real en frecuencias Graves, Medias y Agudas usando filtros IIR ultraligeros.

### Ejecución

- **Iniciar simulador:**
  ```bash
  cargo run --release --bin particles
  ```
  *(O usando el ejecutable directo: `.\target\release\particles.exe`)*

### Modos Visuales (Presiona `V` para alternar)

- **`Particles (Gravity Flow)`**: Modo de partículas flotantes libres. Reaccionan a la gravedad y flotan con los bajos de la música.
- **`Liquid Constellation (Fluid Web)`**: Dibuja líneas de unión inteligentes entre partículas cercanas. Se ve como una red neuronal o una gota de líquido que vibra con el sonido.
- **`Ring Spectrum (Path Snapping)`**: Las partículas se ordenan formando un aro perfecto. El aro late con los graves y se ondula con los medios. Al hacer clic, las partículas se estiran elásticamente hacia el cursor y luego rebotan hacia su órbita al soltar el clic.
- **`Oscilloscope Waveform (Flow Wave)`**: Las partículas se ordenan como una línea de osciloscopio que fluye horizontalmente, modelando la frecuencia del audio en tiempo real con elasticidad elástica al ratón.

### Tabla de Atajos y Controles de Teclado

| Tecla / Control | Acción |
| :--- | :--- |
| **`Click Izquierdo` (Mantener)** | Activa el punto de gravedad (atracción) en el cursor. |
| **`Espacio`** | Alterna el modo de gravedad entre **Atracción (verde)** y **Repulsión (rojo)**. |
| **`V`** | Cambia cíclicamente el **Modo Visual** (*Particles, Liquid, Ring, Waveform*). |
| **`C`** | Cambia cíclicamente la **Paleta de Colores** (*Cyberpunk, Rainbow, Volcano, Matrix*). |
| **`M`** | Activa/Desactiva el **Silenciado de Audio** (pausa la reactividad al sonido). |
| **`R`** | Reinicia la posición y velocidad de todas las partículas. |
| **`Flecha Arriba / Abajo`** | Aumenta o disminuye la **Fuerza de la Gravedad**. |
| **`Flecha Izquierda / Derecha`** | Ajusta la **Fricción/Amortiguación** de movimiento (deslizar vs detener rápido). |
| **`Teclas 1, 2, 3, 4`** | Cambia el total de partículas a **1000, 3000, 5000 o 10000** (en los modos libres). |

---

## Detalles de Eficiencia (Rust Performance)

Ambas aplicaciones fueron diseñadas para consumir una cantidad minúscula de recursos:
- **CPU**: El procesador de audio loopback e IIR realiza ~300k operaciones por segundo (menos de 2% de uso en un núcleo).
- **RAM**: El simulador de partículas solo consume unos **25 MB - 35 MB** de RAM (principalmente debido al contexto de ventana OpenGL). El convertidor ASCII consume apenas **6 MB - 10 MB** y se descarga de la memoria al terminar de ejecutarse.
- **GPU**: Renderizado 2D acelerado nativamente mediante OpenGL, con consumos menores a 1% de carga en tarjetas gráficas modernas.
