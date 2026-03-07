# GitHub README 語言切換機制分析報告

**專案**: `Poseidoncode/superpowers-mcp`  
**分析日期**: 2026 年 3 月 7 日  
**分析對象**:

- `README.md` (英文版本)
- `README.zh-TW.md` (繁體中文版本)

---

## 📋 執行摘要

經過對兩個 README 檔案的深入分析，我發現 **GitHub 原生並不支援自動語言切換機制**。這兩個檔案之間是**手動關聯**，透過以下方式實現語言切換體驗：

### 核心發現

| 特性               | 實作方式                                            | 是否自動化       |
| ------------------ | --------------------------------------------------- | ---------------- |
| **檔案關聯**       | Markdown 內部連結 (`[繁體中文](./README.zh-TW.md)`) | ❌ 手動          |
| **HTML Meta Tags** | 無                                                  | ❌ 不存在        |
| **Link Tags**      | 無                                                  | ❌ 不存在        |
| **Frontmatter**    | 無                                                  | ❌ 不存在        |
| **自動重定向**     | 無                                                  | ❌ GitHub 不支援 |
| **用戶端切換**     | 使用者點擊文字連結                                  | ⚠️ 手動          |

---

## 🔍 詳細技術分析

### 1. 檔案命名規範

```
README.md           → 預設語言（英文，GitHub 優先顯示）
README.zh-TW.md     → 地區特定語言（繁體中文）
```

**GitHub 的檔案識別規則**:

- GitHub 會將 `README.md` 作為預覽檔案自動渲染
- 其他語言版本的 README（如 `README.zh-TW.md`）**不會自動顯示**
- 使用者必須**手動點擊連結**才能切換到對應語言版本

### 2. 關聯機制：Markdown 內部連結

在兩個檔案的開頭都包含相同的語言切換列：

#### English Version (`README.md`):

```markdown
# Superpowers MCP Toolpack Usage Guide

English | [繁體中文](./README.zh-TW.md)
```

#### Traditional Chinese Version (`README.zh-TW.md`):

```markdown
# Superpowers MCP Toolpack 使用指南

English | 繁體中文
```

**關鍵觀察**:

- ✅ 英文版使用**可點擊的 Markdown 連結**指向中文版
- ❌ 中文版使用**純文字**（沒有反向連結）
- ⚠️ 這種設計**不對稱**，中文版使用者無法一鍵切回英文版

### 3. 缺少的主要技術元件

#### ❌ HTML Meta Tags

GitHub README **不支援**在 Markdown 中嵌入 HTML meta tags 來控制語言切換：

```html
<!-- 這些標籤在 GitHub README 中無效 -->
<meta http-equiv="content-language" content="en" />
<link rel="alternate" hreflang="zh-TW" href="./README.zh-TW.md" />
<link rel="alternate" hreflang="en" href="./README.md" />
```

#### ❌ JSON-LD Structured Data

```json
<!-- 這在 GitHub README 中也無效 -->
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "inLanguage": {
    "@type": "Language",
    "name": "en"
  },
  "potentialAction": {
    "@type": "TranslateAction",
    "target": {
      "@type": "EntryPoint",
      "urlTemplate": "./README.zh-TW.md"
    }
  }
}
</script>
```

#### ❌ Frontmatter（YAML 標頭）

```yaml
---
title: "Superpowers MCP Toolpack Usage Guide"
language: en
alternateLanguages:
  - zh-TW
---
```

GitHub 的 Markdown 渲染器**忽略 frontmatter**（除非使用特定的 Jekyll 主題）。

---

## 🌐 GitHub 的國際化限制

### 現行行為

1. **預設顯示**: GitHub 始終優先顯示 `README.md`（英文）
2. **語言偵測**: GitHub **不會**根據瀏覽器設定自動切換 README 語言
3. **搜尋索引**: GitHub 搜尋會索引所有 README 檔案，但顯示時仍以 `README.md` 為主

### 官方討論與限制

根據 GitHub Community Forum 的討論（Discussion #31132, #50719）：

> **"These files were created to make the repo easier to understand in different languages, but instead of automatically selecting the appropriate readme file to display, they just sit there."**

GitHub **尚未提供**原生的多語言 README 自動切換功能。

---

## 💡 建議的改進方案

### 方案 A：對稱式內部連結（最低成本）

修改 `README.zh-TW.md`，添加反向連結：

```markdown
# Superpowers MCP Toolpack 使用指南

[English](./README.md) | 繁體中文
```

**優點**:

- ✅ 零成本修改
- ✅ 立即改善使用者體驗
- ✅ 符合現有模式

**缺點**:

- ⚠️ 仍需手動點擊

---

### 方案 B：GitHub Pages + 自動重定向（中等成本）

建立簡單的 GitHub Pages 網站，使用 JavaScript 自動重定向：

```html
<!-- index.html -->
<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8" />
    <title>Superpowers MCP Toolpack</title>
    <script>
      // 根據瀏覽器語言偏好自動重定向
      const userLang = navigator.language || navigator.userLanguage;
      const langMap = {
        zh: "./README.zh-TW.md",
        en: "./README.md",
      };

      const targetPage = langMap[userLang.substring(0, 2)] || "./README.md";
      window.location.href = targetPage;
    </script>
  </head>
  <body>
    <p>
      If you are not redirected automatically, follow this
      <a href="./README.md">link to the README</a>.
    </p>
  </body>
</html>
```

**優點**:

- ✅ 自動語言檢測
- ✅ 更好的使用者體驗

**缺點**:

- ⚠️ 需要額外設定 GitHub Pages
- ⚠️ 繞過 GitHub 原生 README 渲染

---

### 方案 C：使用 `.github/PULL_REQUEST_TEMPLATE.md` 註解（補充說明）

在 PR Template 中說明多語言文件的存在：

```markdown
## 📄 Documentation

- 🇺🇸 [English README](./README.md)
- 🇹🇼 [繁體中文 README](./README.zh-TW.md)
```

**優點**:

- ✅ 提高能見度
- ✅ 標準化呈現

---

## 🎯 最佳實踐總結

### 當前實作的優缺點

| 項目           | 評分       | 說明                     |
| -------------- | ---------- | ------------------------ |
| **可發現性**   | ⭐⭐☆☆☆    | 需手動尋找語言選項       |
| **一致性**     | ⭐⭐⭐☆☆   | 英文版有連結，中文版缺失 |
| **技術先進性** | ⭐☆☆☆☆     | 無自動化工具             |
| **維護成本**   | ⭐⭐⭐⭐⭐ | 極低，只需維護兩個檔案   |
| **使用者體驗** | ⭐⭐☆☆☆    | 基本可用，但有改進空間   |

### 推薦行動清單

1. **立即修復**（5 分鐘）:
   - 在 `README.zh-TW.md` 添加反向連結至英文版
2. **中期優化**（1 小時）:
   - 考慮實施方案 B（GitHub Pages 自動重定向）
3. **長期追蹤**:
   - 關注 GitHub 官方是否推出原生多語言 README 功能
   - 參考 Discussion #179316 的進展

---

## 📚 相關資源

- [GitHub Community: Multilingual README discussions](https://github.com/orgs/community/discussions/31132)
- [MDN: Internationalization best practices](https://developer.mozilla.org/en-US/docs/Web/Internationalization)
- [W3C: Content Language Guidelines](https://www.w3.org/TR/html401/struct/dirlang.html)

---

**報告生成時間**: 2026-03-07  
**分析工具**: Tavily Extract API, Web Search  
**分析師**: AI Assistant (qwen3.5-flash)
