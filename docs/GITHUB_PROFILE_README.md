<div align="center">

  <!-- Typing SVG Banner -->
  <a href="https://github.com/BakwanJagungg">
    <img src="https://readme-typing-svg.demolab.com?font=Fira+Code&weight=600&size=24&pause=1000&color=0284C7&center=true&vCenter=true&width=600&lines=Hi+there,+I'm+Dava!+👋;Software+Engineer+%26+Automation+Builder;Building+High-Impact+Scientific+Tools;Welcome+to+my+GitHub+Profile!+🚀" alt="Typing SVG" />
  </a>

  <p align="center">
    <b>Passionate Software Engineer specializing in Python, System Automation, and Hybrid AI Solutions.</b><br>
    <i>Crafting lightweight, 100% local, high-precision desktop software for students and developers.</i>
  </p>

  <!-- Badges -->
  <p align="center">
    <a href="https://github.com/BakwanJagungg"><img src="https://img.shields.io/badge/Python-3776AB?style=for-the-badge&logo=python&logoColor=white" alt="Python"></a>
    <a href="https://github.com/BakwanJagungg"><img src="https://img.shields.io/badge/CustomTkinter-059669?style=for-the-badge&logo=gnome&logoColor=white" alt="CustomTkinter"></a>
    <a href="https://github.com/BakwanJagungg"><img src="https://img.shields.io/badge/Linux_WSL-FCC624?style=for-the-badge&logo=linux&logoColor=black" alt="Linux WSL"></a>
    <a href="https://github.com/BakwanJagungg"><img src="https://img.shields.io/badge/Local_AI-Ollama-black?style=for-the-badge&logo=ollama&logoColor=white" alt="Ollama"></a>
    <a href="https://github.com/BakwanJagungg"><img src="https://img.shields.io/badge/SQLite_WAL-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite"></a>
  </p>

</div>

---

### 🚀 Sedang Dikerjakan & Proyek Unggulan

- 🔭 **Proyek Utama**: **[JSFS (Journal Scraper For Students)](https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-)**  
  *Desktop Research Assistant* cerdas untuk otomasi pencarian dan ekstraksi jurnal ilmiah terakreditasi SINTA 1–6 & Portal Garuda. Dilengkapi mesin *Deep APC Hunter*, *Forensic Data Purity*, dan generator daftar 1-klik siap share WhatsApp/Telegram.
- 💡 **Fokus Riset**: Ekstraksi data web deterministik (*Human-Parity Accuracy*), optimasi memori desktop GUI (Zero-Crash & Zero-Freeze), dan integrasi Model Bahasa Lokal (Ollama qwen2.5 / phi3.5).
- 🌱 **Eksplorasi Teknologi**: Arsitektur Desktop Python modern, Linux WSL 2, SQLite Write-Ahead Logging (WAL), dan otomasi CI/CD GitHub Actions.
- 💬 **Tanya Saya Tentang**: Python Web Scraping, pembuatan biner `.exe` portabel mandiri via PyInstaller, serta trik integrasi data OJS kampus.

---

### 🛠️ Tech Stack & Ekosistem Keahlian

<table align="center" width="100%">
  <tr>
    <td align="center" width="20%"><b>Bahasa Pemrograman</b></td>
    <td align="center" width="20%"><b>GUI & Desktop UI</b></td>
    <td align="center" width="20%"><b>Data & Database</b></td>
    <td align="center" width="20%"><b>AI & LLM Tools</b></td>
    <td align="center" width="20%"><b>Lingkungan & DevOps</b></td>
  </tr>
  <tr>
    <td align="center" valign="top">
      <img src="https://skillicons.dev/icons?i=python,bash" alt="Languages" /><br>
      <sub>Python · Bash / Shell</sub>
    </td>
    <td align="center" valign="top">
      <img src="https://skillicons.dev/icons?i=qt" alt="UI" /><br>
      <sub>CustomTkinter · Tkinter</sub>
    </td>
    <td align="center" valign="top">
      <img src="https://skillicons.dev/icons?i=sqlite,postgres" alt="Databases" /><br>
      <sub>SQLite (WAL) · Pandas</sub>
    </td>
    <td align="center" valign="top">
      <img src="https://skillicons.dev/icons?i=pytorch,ollama" alt="AI" /><br>
      <sub>Ollama (Qwen/Phi) · BeautifulSoup</sub>
    </td>
    <td align="center" valign="top">
      <img src="https://skillicons.dev/icons?i=linux,ubuntu,git,github,vscode" alt="Tools" /><br>
      <sub>WSL 2 · Ubuntu · Git · VS Code</sub>
    </td>
  </tr>
</table>

---

### 📊 Statistik GitHub & Aktivitas Riset

<div align="center">

  <!-- GitHub Stats Card -->
  <img src="https://github-readme-stats.vercel.app/api?username=BakwanJagungg&show_icons=true&theme=radical&hide_border=true&include_all_commits=true&count_private=true" alt="Statistik GitHub Dava" width="48%" />

  <!-- Top Languages Card -->
  <img src="https://github-readme-stats.vercel.app/api/top-langs/?username=BakwanJagungg&layout=compact&theme=radical&hide_border=true" alt="Bahasa Pemrograman Teratas" width="48%" />

  <br><br>

  <!-- GitHub Streak Card -->
  <img src="https://github-readme-streak-stats.herokuapp.com/?user=BakwanJagungg&theme=radical&hide_border=true" alt="GitHub Streak Dava" width="97%" />

</div>

---

### 🐍 GitHub Contribution Snake Animation

<div align="center">
  <img src="https://raw.githubusercontent.com/BakwanJagungg/BakwanJagungg/output/github-contribution-grid-snake.svg" alt="Snake animation" width="100%" />
</div>

<details>
<summary><b>💡 Cara Mengaktifkan Animasi Ular (Snake) di Profil Anda</b></summary>

Cukup buat file workflow di repositori profil Anda: `.github/workflows/snake.yml`:
```yaml
name: Generate Snake Animation

on:
  schedule:
    - cron: "0 0 * * *" # Berjalan otomatis setiap hari
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: Platane/snk@v3
        with:
          github_user_name: BakwanJagungg
          outputs: |
            dist/github-contribution-grid-snake.svg
            dist/github-contribution-grid-snake-dark.svg?palette=github-dark
      - uses: crazy-max/ghaction-github-pages@v3.1.0
        with:
          target_branch: output
          build_dir: dist
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```
</details>

---

### 📬 Mari Terhubung & Berkolaborasi

<div align="center">

  <a href="https://github.com/BakwanJagungg">
    <img src="https://img.shields.io/badge/GitHub-Profile-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub" />
  </a>
  &nbsp;&nbsp;
  <a href="https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-/issues">
    <img src="https://img.shields.io/badge/Feedback-JSFS_Project-0284c7?style=for-the-badge&logo=gitbook&logoColor=white" alt="JSFS Feedback" />
  </a>

  <br><br>
  <sub>*Didedikasikan untuk pengembangan perangkat lunak open-source yang bermanfaat bagi dunia riset dan akademisi.*</sub>

</div>
