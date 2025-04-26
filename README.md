# LaTeX CV Template

A clean, professional, and customizable CV/resume template created with LaTeX.

![CV Preview](cv-preview.png)

## Language Versions

- **English**: Main branch (current)
- **Polish**: Available in the `cv-pl` branch

## Prerequisites

- A LaTeX distribution (e.g., TeX Live, MiKTeX)
- A LaTeX editor (e.g., TeXstudio, Overleaf)

## Getting Started

1. Open `cv.tex` in your LaTeX editor
2. Replace the placeholder information with your personal details
3. Compile the document to generate a PDF

## Customization

### Color Scheme

You can change the primary color by modifying the RGB values in:

```tex
\definecolor{primary}{RGB}{70, 130, 180} % Steel Blue
```

### Sections

Add, remove, or rearrange sections as needed. Each section follows this pattern:

```tex
\section{Section Name}
\cvEntry{Title}{Location}{Subtitle}{Date Range}
\begin{itemize}[leftmargin=*]
  \cvSubItem{Description point 1}
  \cvSubItem{Description point 2}
\end{itemize}
```

### Photo (Optional)

To add a photo, use the `graphicx` package (already included) and add this code after the header:

```tex
\begin{center}
  \includegraphics[width=2.5cm]{photo.jpg}
\end{center}
```

## Tips for Creating a Great CV

1. **Keep it concise**: Aim for 1-2 pages maximum
2. **Quantify achievements**: Use numbers to showcase your impact
3. **Use action verbs**: Begin bullet points with strong action verbs
4. **Customize for each application**: Highlight relevant skills/experiences
5. **Proofread carefully**: Check for typos and formatting issues

## License

This template is available under the MIT License. See the [LICENSE](LICENSE) file for details.

Photo source: https://www.pexels.com/pl-pl/zdjecie/mezczyzna-na-portret-szarej-koszuli-91227/