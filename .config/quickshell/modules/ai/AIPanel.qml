pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 480
    implicitHeight: 640
    focus: true

    // --- State Properties ---
    property bool showSettings: false
    property bool isGenerating: false
    property string activeProvider: "gemini" // gemini | openai | claude | custom | ollama
    property string activeModel: "gemini-2.5-flash"
    property string systemPersona: "General Assistant"
    property string statusMessage: ""
    property string keyDetectStatus: ""

    // Universal Key Input
    property string universalKeyInput: ""

    // API Keys and Endpoints (persisted in ~/.config/quickshell/ai_config.json)
    property string geminiKey: ""
    property string openaiKey: ""
    property string claudeKey: ""
    property string ollamaHost: "http://localhost:11434"
    property string customEndpoint: "https://api.groq.com/openai/v1/chat/completions"
    property string customKey: ""
    property string groqKey: ""
    property string openrouterKey: ""
    property string deepseekKey: ""
    property string mistralKey: ""
    property string xaiKey: ""
    property string togetherKey: ""
    property string perplexityKey: ""
    property string zhipuKey: ""
    property string siliconflowKey: ""
    property string moonshotKey: ""
    property string qwenKey: ""
    property string fireworksKey: ""

    // Simulated Browser session (web login instead of API key)
    property bool browserMode: false
    property string browserProvider: "gemini"
    property string browserEndpoint: ""
    property string browserSession: ""
    property string browserUserAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
    property string browserStatus: ""

    // Chat History Array — capped to prevent unbounded memory growth
    // that causes the shell to become unresponsive over time.
    property var messages: []
    property int maxMessages: 200

    // Signal to notify parent or shell to close panel
    signal requestClose()

    // --- Helper Functions ---
    function autoDetectKey(keyText) {
        const k = String(keyText || "").trim()
        universalKeyInput = k
        if (k.length === 0) {
            keyDetectStatus = ""
            return
        }

        // If the pasted key already belongs to the active provider, keep it.
        const owned = activeProvider === "gemini" ? (k === geminiKey)
            : activeProvider === "openai" ? (k === openaiKey)
            : activeProvider === "claude" ? (k === claudeKey)
            : activeProvider === "groq" ? (k === groqKey)
            : activeProvider === "openrouter" ? (k === openrouterKey)
            : activeProvider === "deepseek" ? (k === deepseekKey)
            : activeProvider === "mistral" ? (k === mistralKey)
            : activeProvider === "xai" ? (k === xaiKey)
            : activeProvider === "together" ? (k === togetherKey)
            : activeProvider === "perplexity" ? (k === perplexityKey)
            : activeProvider === "zhipu" ? (k === zhipuKey)
            : activeProvider === "siliconflow" ? (k === siliconflowKey)
            : activeProvider === "moonshot" ? (k === moonshotKey)
            : activeProvider === "qwen" ? (k === qwenKey)
            : activeProvider === "fireworks" ? (k === fireworksKey)
            : activeProvider === "custom" ? (k === customKey)
            : false
        if (owned) return

        if (k.indexOf("AIzaSy") === 0) {
            activeProvider = "gemini"
            activeModel = "gemini-2.5-flash"
            geminiKey = k
            keyDetectStatus = "✨ Auto-detected: Google Gemini API Key"
        } else if (k.indexOf("sk-ant-") === 0) {
            activeProvider = "claude"
            activeModel = "claude-3-5-sonnet-latest"
            claudeKey = k
            keyDetectStatus = "✨ Auto-detected: Anthropic Claude API Key"
        } else if (k.indexOf("gsk_") === 0) {
            activeProvider = "groq"
            activeModel = "llama-3.3-70b-versatile"
            customEndpoint = "https://api.groq.com/openai/v1/chat/completions"
            groqKey = k
            keyDetectStatus = "✨ Auto-detected: Groq API Key"
        } else if (k.indexOf("sk-or-") === 0) {
            activeProvider = "openrouter"
            activeModel = "meta-llama/llama-3.3-70b-instruct:free"
            customEndpoint = "https://openrouter.ai/api/v1/chat/completions"
            openrouterKey = k
            keyDetectStatus = "✨ Auto-detected: OpenRouter API Key"
        } else if (k.indexOf("xai-") === 0) {
            activeProvider = "xai"
            activeModel = "grok-2-latest"
            customEndpoint = "https://api.x.ai/v1/chat/completions"
            xaiKey = k
            keyDetectStatus = "✨ Auto-detected: xAI Grok API Key"
        } else if (k.indexOf("pplx-") === 0) {
            activeProvider = "perplexity"
            activeModel = "sonar"
            customEndpoint = "https://api.perplexity.ai/chat/completions"
            perplexityKey = k
            keyDetectStatus = "✨ Auto-detected: Perplexity API Key"
        } else if (k.indexOf("sk-") === 0) {
            activeProvider = "openai"
            activeModel = "gpt-4o-mini"
            openaiKey = k
            keyDetectStatus = "✨ Auto-detected: OpenAI ChatGPT API Key (DeepSeek/Mistral sk- keys: pick the provider button)"
        } else {
            keyDetectStatus = "💡 Custom Key / Manual Provider"
        }
    }

    function appendMessage(sender, text) {
        const next = messages.slice()
        const now = new Date()
        const timeStr = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
        next.push({
            sender: sender,
            text: text,
            timestamp: timeStr
        })
        // Cap to prevent unbounded memory growth
        if (next.length > maxMessages) {
            next.splice(0, next.length - maxMessages)
        }
        messages = next
        Qt.callLater(function() {
            if (chatListView)
                chatListView.positionViewAtEnd()
        })
    }

    function updateLastMessage(text) {
        if (messages.length === 0) return
        const next = messages.slice()
        next[next.length - 1].text = text
        messages = next
        Qt.callLater(function() {
            if (chatListView)
                chatListView.positionViewAtEnd()
        })
    }

    function clearChat() {
        messages = []
        statusMessage = ""
    }

    function copyToClipboard(text) {
        copyProc.command = ["bash", "-c", "cat << 'EOF' | wl-copy\n" + text + "\nEOF"]
        copyProc.running = true
        statusMessage = "Copied to clipboard!"
    }

    function pasteClipboardToSend() {
        pasteProc.running = true
    }

    function parseConfig(jsonText) {
        try {
            if (!jsonText || jsonText.trim().length === 0) return
            const cfg = JSON.parse(jsonText)
            if (cfg.provider) activeProvider = cfg.provider
            if (cfg.provider === "browser" && cfg.browserProvider) {
                browserMode = true
                activeProvider = cfg.browserProvider
            }
            if (cfg.model) activeModel = cfg.model
            if (cfg.geminiKey !== undefined) geminiKey = cfg.geminiKey
            if (cfg.openaiKey !== undefined) openaiKey = cfg.openaiKey
            if (cfg.claudeKey !== undefined) claudeKey = cfg.claudeKey
            if (cfg.ollamaHost !== undefined) ollamaHost = cfg.ollamaHost
            if (cfg.customEndpoint !== undefined) customEndpoint = cfg.customEndpoint
            if (cfg.customKey !== undefined) customKey = cfg.customKey
            if (cfg.groqKey !== undefined) groqKey = cfg.groqKey
            if (cfg.openrouterKey !== undefined) openrouterKey = cfg.openrouterKey
            if (cfg.deepseekKey !== undefined) deepseekKey = cfg.deepseekKey
            if (cfg.mistralKey !== undefined) mistralKey = cfg.mistralKey
            if (cfg.xaiKey !== undefined) xaiKey = cfg.xaiKey
            if (cfg.togetherKey !== undefined) togetherKey = cfg.togetherKey
            if (cfg.perplexityKey !== undefined) perplexityKey = cfg.perplexityKey
            if (cfg.zhipuKey !== undefined) zhipuKey = cfg.zhipuKey
            if (cfg.siliconflowKey !== undefined) siliconflowKey = cfg.siliconflowKey
            if (cfg.moonshotKey !== undefined) moonshotKey = cfg.moonshotKey
            if (cfg.qwenKey !== undefined) qwenKey = cfg.qwenKey
            if (cfg.fireworksKey !== undefined) fireworksKey = cfg.fireworksKey
            if (cfg.browserMode !== undefined) browserMode = cfg.browserMode === true || cfg.browserMode === "true"
            if (cfg.browserProvider !== undefined) browserProvider = cfg.browserProvider
            if (cfg.browserEndpoint !== undefined) browserEndpoint = cfg.browserEndpoint
            if (cfg.browserSession !== undefined) browserSession = cfg.browserSession
            if (cfg.browserUserAgent !== undefined) browserUserAgent = cfg.browserUserAgent
            if (cfg.systemPersona !== undefined) systemPersona = cfg.systemPersona

            // Pre-fill universal key box if active key exists
            if (activeProvider === "gemini" && geminiKey) universalKeyInput = geminiKey
            else if (activeProvider === "openai" && openaiKey) universalKeyInput = openaiKey
            else if (activeProvider === "claude" && claudeKey) universalKeyInput = claudeKey
            else if (activeProvider === "groq" && groqKey) universalKeyInput = groqKey
            else if (activeProvider === "openrouter" && openrouterKey) universalKeyInput = openrouterKey
            else if (activeProvider === "deepseek" && deepseekKey) universalKeyInput = deepseekKey
            else if (activeProvider === "mistral" && mistralKey) universalKeyInput = mistralKey
            else if (activeProvider === "xai" && xaiKey) universalKeyInput = xaiKey
            else if (activeProvider === "together" && togetherKey) universalKeyInput = togetherKey
            else if (activeProvider === "perplexity" && perplexityKey) universalKeyInput = perplexityKey
            else if (activeProvider === "zhipu" && zhipuKey) universalKeyInput = zhipuKey
            else if (activeProvider === "siliconflow" && siliconflowKey) universalKeyInput = siliconflowKey
            else if (activeProvider === "moonshot" && moonshotKey) universalKeyInput = moonshotKey
            else if (activeProvider === "qwen" && qwenKey) universalKeyInput = qwenKey
            else if (activeProvider === "fireworks" && fireworksKey) universalKeyInput = fireworksKey
            else if (activeProvider === "custom" && customKey) universalKeyInput = customKey
        } catch(e) {
            console.log("Error parsing AI config:", e)
        }
    }

    function buildConfigJson() {
        const cfg = {
            provider: browserMode ? "browser" : activeProvider,
            model: activeModel,
            geminiKey: geminiKey,
            openaiKey: openaiKey,
            claudeKey: claudeKey,
            ollamaHost: ollamaHost,
            customEndpoint: customEndpoint,
            customKey: customKey,
            groqKey: groqKey,
            openrouterKey: openrouterKey,
            deepseekKey: deepseekKey,
            mistralKey: mistralKey,
            xaiKey: xaiKey,
            togetherKey: togetherKey,
            perplexityKey: perplexityKey,
            zhipuKey: zhipuKey,
            siliconflowKey: siliconflowKey,
            moonshotKey: moonshotKey,
            qwenKey: qwenKey,
            fireworksKey: fireworksKey,
            systemPersona: systemPersona,
            browserMode: browserMode,
            browserProvider: browserProvider,
            browserEndpoint: browserEndpoint,
            browserSession: browserSession,
            browserUserAgent: browserUserAgent
        }
        return JSON.stringify(cfg, null, 2)
    }

    function saveConfig() {
        const str = buildConfigJson()
        saveConfigProc.command = ["bash", "-c", "mkdir -p ~/.config/quickshell && cat << 'EOF' > ~/.config/quickshell/ai_config.json\n" + str + "\nEOF\n"]
        saveConfigProc.running = true
        statusMessage = "Settings saved!"
    }

    // --- Request builder (replaces the external qs-backend script) ---

    // Wrap a JSON body + curl flags into a command that pipes the body to curl
    // on stdin through a quoted heredoc (no shell expansion of the payload).
    function heredocCmd(body, curlArgs) {
        let delim = "QS_AI_BODY_7x9"
        while (String(body).indexOf(delim) !== -1) delim += "x"
        return "curl -sS " + curlArgs + " --data-binary @- << '" + delim + "'\n" + body + "\n" + delim
    }

    // Extract endpoint + Cookie header from a pasted DevTools "Copy as cURL".
    function parseCurlSession(raw) {
        const s = String(raw || "")
        const quote = s.indexOf("'") !== -1 ? "'" : '"'
        let ep = ""
        let cookie = ""
        const curlPos = s.indexOf("curl")
        if (curlPos !== -1) {
            const q1 = s.indexOf(quote, curlPos + 4)
            const q2 = s.indexOf(quote, q1 + 1)
            if (q1 !== -1 && q2 > q1) ep = s.slice(q1 + 1, q2)
        }
        const hRe = /-H\s+['"]([^'"]*)['"]/g
        let m
        while ((m = hRe.exec(s)) !== null) {
            const h = m[1]
            const colon = h.indexOf(":")
            if (colon !== -1 && h.slice(0, colon).trim().toLowerCase() === "cookie") {
                cookie = h.slice(colon + 1).trim()
            }
        }
        return { ep: ep, cookie: cookie }
    }

    // Endpoint + key + default model for the OpenAI-compatible providers.
    function providerConf(p) {
        switch (p) {
        case "openai": return { url: "https://api.openai.com/v1/chat/completions", key: openaiKey, model: "gpt-4o-mini" }
        case "groq": return { url: "https://api.groq.com/openai/v1/chat/completions", key: groqKey, model: "llama-3.3-70b-versatile" }
        case "openrouter": return { url: "https://openrouter.ai/api/v1/chat/completions", key: openrouterKey, model: "meta-llama/llama-3.3-70b-instruct:free" }
        case "deepseek": return { url: "https://api.deepseek.com/chat/completions", key: deepseekKey, model: "deepseek-chat" }
        case "mistral": return { url: "https://api.mistral.ai/v1/chat/completions", key: mistralKey, model: "mistral-small-latest" }
        case "xai": return { url: "https://api.x.ai/v1/chat/completions", key: xaiKey, model: "grok-2-latest" }
        case "together": return { url: "https://api.together.xyz/v1/chat/completions", key: togetherKey, model: "meta-llama/Llama-3.3-70B-Instruct-Turbo" }
        case "perplexity": return { url: "https://api.perplexity.ai/chat/completions", key: perplexityKey, model: "sonar" }
        case "zhipu": return { url: "https://open.bigmodel.cn/api/paas/v4/chat/completions", key: zhipuKey, model: "glm-4-flash" }
        case "siliconflow": return { url: "https://api.siliconflow.cn/v1/chat/completions", key: siliconflowKey, model: "Qwen/Qwen2.5-7B-Instruct" }
        case "moonshot": return { url: "https://api.moonshot.cn/v1/chat/completions", key: moonshotKey, model: "moonshot-v1-8k" }
        case "qwen": return { url: "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions", key: qwenKey, model: "qwen-plus" }
        case "fireworks": return { url: "https://api.fireworks.ai/inference/v1/chat/completions", key: fireworksKey, model: "accounts/fireworks/models/llama-v3p1-8b-instruct" }
        default: return { url: "", key: "", model: "" }
        }
    }

    // Build the {command, error} for the active provider/mode. The prompt text
    // is embedded into the JSON payload via JSON.stringify, never into the shell.
    function buildRequest(promptPayload) {
        const sys = "You are a helpful AI assistant."
        const msgs = [ { role: "system", content: sys }, { role: "user", content: promptPayload } ]

        if (browserMode) {
            let ep = String(browserEndpoint || "").trim()
            let ses = String(browserSession || "").trim()
            if (ses.indexOf("curl") === 0) {
                const parsed = parseCurlSession(ses)
                if (parsed.ep) ep = parsed.ep
                if (parsed.cookie) ses = parsed.cookie
            }
            if (!ep) return { command: "", error: "No browser endpoint set. In Settings > Simulate Browser, paste a web endpoint." }
            if (!ses) return { command: "", error: "No browser session saved. In Settings > Simulate Browser, log in, then paste your cookies/session." }
            const body = JSON.stringify({ model: activeModel, messages: [ { role: "user", content: promptPayload } ] })
            const args = "-X POST '" + ep + "' -H 'Content-Type: application/json' -H \"User-Agent: " + browserUserAgent + "\" -H \"Cookie: " + ses + "\""
            return { command: heredocCmd(body, args), error: "" }
        }

        switch (activeProvider) {
        case "gemini": {
            const model = activeModel || "gemini-2.5-flash"
            const key = String(geminiKey || "").trim()
            if (!key) return { command: "", error: "Gemini API Key missing." }
            const url = "https://generativelanguage.googleapis.com/v1beta/models/" + model + ":generateContent?key=" + key
            const body = JSON.stringify({ contents: [ { role: "user", parts: [ { text: promptPayload } ] } ] })
            return { command: heredocCmd(body, "-X POST '" + url + "' -H 'Content-Type: application/json'"), error: "" }
        }
        case "claude": {
            const model = activeModel || "claude-3-5-sonnet-latest"
            const key = String(claudeKey || "").trim()
            if (!key) return { command: "", error: "Claude API Key missing." }
            const body = JSON.stringify({ model: model, messages: [ { role: "user", content: promptPayload } ], system: sys })
            const args = "-X POST 'https://api.anthropic.com/v1/messages' -H 'Content-Type: application/json' -H 'x-api-key: " + key + "'"
            return { command: heredocCmd(body, args), error: "" }
        }
        case "ollama": {
            const host = String(ollamaHost || "").trim() || "http://localhost:11434"
            const model = activeModel || "llama3"
            const body = JSON.stringify({ model: model, stream: false, messages: msgs })
            return { command: heredocCmd(body, "-X POST '" + host + "/api/chat' -H 'Content-Type: application/json'"), error: "" }
        }
        case "custom": {
            const ep = String(customEndpoint || "").trim()
            const key = String(customKey || "").trim()
            if (!ep) return { command: "", error: "Custom endpoint missing." }
            if (!key) return { command: "", error: "Custom API Key missing." }
            const body = JSON.stringify({ model: activeModel || "llama3-8b-8192", messages: msgs })
            const args = "-X POST '" + ep + "' -H 'Content-Type: application/json' -H 'Authorization: Bearer " + key + "'"
            return { command: heredocCmd(body, args), error: "" }
        }
        case "openai":
        case "groq":
        case "openrouter":
        case "deepseek":
        case "mistral":
        case "xai":
        case "together":
        case "perplexity":
        case "zhipu":
        case "siliconflow":
        case "moonshot":
        case "qwen":
        case "fireworks": {
            const conf = providerConf(activeProvider)
            const key = String(conf.key || "").trim()
            if (!key) return { command: "", error: activeProvider.charAt(0).toUpperCase() + activeProvider.slice(1) + " API Key missing." }
            const body = JSON.stringify({ model: activeModel || conf.model, messages: msgs })
            const args = "-X POST '" + conf.url + "' -H 'Content-Type: application/json' -H 'Authorization: Bearer " + key + "'"
            return { command: heredocCmd(body, args), error: "" }
        }
        default:
            return { command: "", error: "Unsupported provider: " + activeProvider }
        }
    }

    function openBrowserLogin() {
        let url = ""
        if (browserProvider === "gemini") url = "https://gemini.google.com/"
        else if (browserProvider === "openai") url = "https://chatgpt.com/auth/login"
        else if (browserProvider === "claude") url = "https://claude.ai/login"
        else url = browserEndpoint.trim()
        if (url.length === 0) {
            browserStatus = "⚠️ Set a Custom web endpoint first."
            return
        }
        browserProc.command = ["bash", "-c", "xdg-open '" + url + "' 2>/dev/null || true"]
        browserProc.running = true
        browserStatus = "Opened login page in your browser. Log in, then copy a request from DevTools (Network → Copy as cURL) and paste it below."
    }

    function stopGeneration() {
        if (apiProc.running) {
            apiProc.running = false
        }
        isGenerating = false
        statusMessage = "Cancelled"
        updateLastMessage("⏹️ Response cancelled.")
    }

    function sendPrompt(userText) {
        const text = String(userText || "").trim()
        if (text.length === 0 || isGenerating) return

        inputEdit.text = ""
        appendMessage("user", text)
        isGenerating = true
        statusMessage = "Thinking..."

        // Append placeholder for assistant response
        appendMessage("assistant", "...")

        // System prompt setup
        let sysPrompt = "You are a helpful AI assistant."
        if (systemPersona === "Linux & Hyprland Expert") {
            sysPrompt = "You are an expert Linux and Hyprland desktop assistant. Provide concise, clean answers with terminal commands."
        } else if (systemPersona === "Senior Code Developer") {
            sysPrompt = "You are a senior software developer. Provide clean code snippets and direct explanations."
        } else if (systemPersona === "Concise Answerer") {
            sysPrompt = "Answer as concisely as possible without fluff."
        }

        // No external backend script: the request is built here and curl is the
        // only subprocess. The JSON body is fed to curl on stdin via a quoted
        // heredoc, so prompt content can never touch the shell command line.

        // Basic key presence checks to provide immediate user feedback for common providers
        let key = ""
        switch (activeProvider) {
        case "gemini": key = geminiKey.trim() || Quickshell.env("GEMINI_API_KEY") || ""; break
        case "openai": key = openaiKey.trim() || Quickshell.env("OPENAI_API_KEY") || ""; break
        case "claude": key = claudeKey.trim() || Quickshell.env("ANTHROPIC_API_KEY") || ""; break
        case "groq": key = groqKey.trim() || Quickshell.env("GROQ_API_KEY") || ""; break
        case "openrouter": key = openrouterKey.trim() || Quickshell.env("OPENROUTER_API_KEY") || ""; break
        case "deepseek": key = deepseekKey.trim() || Quickshell.env("DEEPSEEK_API_KEY") || ""; break
        case "mistral": key = mistralKey.trim() || Quickshell.env("MISTRAL_API_KEY") || ""; break
        case "xai": key = xaiKey.trim() || Quickshell.env("XAI_API_KEY") || ""; break
        case "together": key = togetherKey.trim() || Quickshell.env("TOGETHER_API_KEY") || ""; break
        case "perplexity": key = perplexityKey.trim() || Quickshell.env("PERPLEXITY_API_KEY") || ""; break
        case "zhipu": key = zhipuKey.trim() || Quickshell.env("ZHIPU_API_KEY") || ""; break
        case "siliconflow": key = siliconflowKey.trim() || Quickshell.env("SILICONFLOW_API_KEY") || ""; break
        case "moonshot": key = moonshotKey.trim() || Quickshell.env("MOONSHOT_API_KEY") || ""; break
        case "qwen": key = qwenKey.trim() || Quickshell.env("DASHSCOPE_API_KEY") || ""; break
        case "fireworks": key = fireworksKey.trim() || Quickshell.env("FIREWORKS_API_KEY") || ""; break
        case "custom": key = customKey.trim() || ""; break
        default: key = ""
        }

        if (!browserMode && activeProvider !== "ollama" && !key) {
            isGenerating = false
            statusMessage = "Missing API Key"
            updateLastMessage("⚠️ " + activeProvider.charAt(0).toUpperCase() + activeProvider.slice(1) + " API Key missing!\n\nPaste your key in ⚙️ Settings — it auto-detects automatically.")
            return
        }
        if (browserMode && browserSession.trim().length === 0) {
            isGenerating = false
            statusMessage = "No Browser Session"
            updateLastMessage("⚠️ No browser session saved.\n\nOpen ⚙️ Settings → Simulate Browser, log in, then paste your cookies and Save Browser Session.")
            return
        }

        const promptPayload = sysPrompt + "\n\nUser Question: " + text
        const req = buildRequest(promptPayload)
        if (req.error) {
            isGenerating = false
            statusMessage = "Request failed"
            updateLastMessage("⚠️ " + req.error)
            return
        }
        apiProc.command = ["bash", "-c", req.command]
        apiProc.running = true
    }

    function handleApiResponse(rawOutput) {
        isGenerating = false
        statusMessage = ""
        if (!rawOutput || rawOutput.trim().length === 0) {
            updateLastMessage("⚠️ Error: Received empty response from API endpoint.")
            return
        }

        try {
            const res = JSON.parse(rawOutput)

            if (res.error) {
                const msg = typeof res.error === "string" ? res.error : (res.error.message || JSON.stringify(res.error))
                updateLastMessage("⚠️ API Error: " + msg)
                return
            }

            let answer = ""
            if (browserMode) {
                if (res.choices && res.choices.length > 0 && res.choices[0].message) {
                    answer = res.choices[0].message.content
                } else if (res.message && res.message.content) {
                    answer = res.message.content
                } else if (res.candidates && res.candidates.length > 0 && res.candidates[0].content && res.candidates[0].content.parts) {
                    answer = res.candidates[0].content.parts.map(function(p) { return p.text || "" }).join("")
                }
            } else if (activeProvider === "gemini") {
                if (res.candidates && res.candidates.length > 0 && res.candidates[0].content && res.candidates[0].content.parts) {
                    answer = res.candidates[0].content.parts.map(function(p) { return p.text || "" }).join("")
                }
            } else if (["openai", "custom", "groq", "openrouter", "deepseek", "mistral", "xai", "together", "perplexity", "zhipu", "siliconflow", "moonshot", "qwen", "fireworks"].indexOf(activeProvider) !== -1) {
                if (res.choices && res.choices.length > 0 && res.choices[0].message) {
                    answer = res.choices[0].message.content
                }
            } else if (activeProvider === "claude") {
                if (res.content && res.content.length > 0) {
                    answer = res.content.map(function(c) { return c.text || "" }).join("")
                }
            } else if (activeProvider === "ollama") {
                if (res.message && res.message.content) {
                    answer = res.message.content
                }
            }

            if (answer && answer.trim().length > 0) {
                updateLastMessage(answer.trim())
            } else {
                updateLastMessage("⚠️ Could not extract text from API response.\n\nRaw output:\n" + rawOutput.substring(0, 250))
            }
        } catch(e) {
            updateLastMessage("⚠️ Output parsing error: " + e.message + "\n\nRaw response:\n" + rawOutput.substring(0, 250))
        }
    }

    // Load config on creation
    Component.onCompleted: {
        loadConfigProc.running = true
    }

    // --- Processes (Event-driven, 0 idle CPU/RAM) ---
    Process {
        id: loadConfigProc
        running: false
        command: ["bash", "-c", "cat ~/.config/quickshell/ai_config.json 2>/dev/null || echo '{}'"]
        stdout: StdioCollector {
            id: loadCollector
            onStreamFinished: function() { root.parseConfig(loadCollector.text) }
        }
    }

    Process {
        id: saveConfigProc
        running: false
    }

    Process {
        id: copyProc
        running: false
    }

    Process {
        id: pasteProc
        running: false
        command: ["bash", "-c", "wl-paste 2>/dev/null"]
        stdout: StdioCollector {
            id: pasteCollector
            onStreamFinished: function() {
                const clipText = String(pasteCollector.text || "").trim()
                if (clipText.length > 0) {
                    if (inputEdit.text.length > 0) {
                        inputEdit.text = inputEdit.text + "\n" + clipText
                    } else {
                        inputEdit.text = clipText
                    }
                }
            }
        }
    }

    Process {
        id: browserProc
        running: false
    }

    Process {
        id: apiProc
        running: false
        stdout: StdioCollector {
            id: apiCollector
            onStreamFinished: function() { root.handleApiResponse(apiCollector.text) }
        }
        onExited: function(exitCode, exitStatus) {
            if (exitCode !== 0 && root.isGenerating) {
                root.isGenerating = false
                root.statusMessage = "Request failed"
                root.updateLastMessage("⚠️ Network or API request failed (Exit code: " + exitCode + "). Check your connection and API key.")
            }
        }
    }

    // --- UI Layout ---
    Rectangle {
        anchors.fill: parent
        radius: 24
        color: Theme.bg
        border.color: Theme.border
        border.width: 1

        Column {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 10

            // --- Header Bar ---
            Rectangle {
                width: parent.width
                height: 44
                radius: 14
                color: Theme.bg2

                Row {
                    anchors.left: parent.left
                    anchors.leftMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 10

                    Rectangle {
                        width: 28
                        height: 28
                        radius: 8
                        color: Theme.acc
                        anchors.verticalCenter: parent.verticalCenter

                        Txt {
                            anchors.centerIn: parent
                            text: "󰚩"
                            color: Theme.sfg
                            font.family: Theme.iconFont
                            font.pixelSize: 15
                        }
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 1

                        Txt {
                            text: "AI Assistant"
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 14
                            font.bold: true
                        }

                        Txt {
                            text: (root.browserMode ? "🖥️ " : "") + root.activeProvider.toUpperCase() + " • " + root.activeModel
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 10
                        }
                    }
                }

                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 6

                    // Settings Toggle Button
                    IconButton {
                        glyph: "󰒓"
                        size: 32
                        glyphSize: 14
                        selected: root.showSettings
                        selectedFill: Theme.acc
                        onClicked: root.showSettings = !root.showSettings
                    }

                    // Clear Chat Button
                    IconButton {
                        glyph: "󰎟"
                        size: 32
                        glyphSize: 14
                        color: Theme.fg2
                        onClicked: root.clearChat()
                    }

                    // Close Button
                    IconButton {
                        glyph: "󰅖"
                        size: 32
                        glyphSize: 14
                        color: Theme.danger
                        onClicked: {
                            if (typeof StateController !== "undefined" && StateController.pillReset) {
                                StateController.pillReset()
                            } else {
                                root.requestClose()
                            }
                        }
                    }
                }
            }

            // --- Main Content Switcher ---
            Item {
                width: parent.width
                height: parent.height - 120

                // === View 1: Settings Drawer ===
                Flickable {
                    id: settingsFlick
                    anchors.fill: parent
                    visible: root.showSettings
                    contentHeight: settingsCol.height + 20
                    clip: true

                    Column {
                        id: settingsCol
                        width: parent.width
                        spacing: 12

                        Txt {
                            text: "⚙️ AI Auto-Detection & Key Settings"
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 14
                            font.bold: true
                        }

                        // === Universal Auto-Detect Key Box ===
                        Column {
                            width: parent.width
                            spacing: 6

                            Txt {
                                text: "🔑 Paste ANY API Key (Auto-Detects Gemini, OpenAI, Claude, Groq, OpenRouter):"
                                color: Theme.info
                                font.pixelSize: 11
                                font.bold: true
                            }

                            Rectangle {
                                width: parent.width
                                height: 38
                                radius: 10
                                color: Theme.bg2
                                border.color: root.keyDetectStatus.indexOf("Auto-detected") !== -1 ? Theme.success : Theme.border
                                border.width: 1

                                TxtInput {
                                    id: keyBox
                                    anchors.fill: parent
                                    anchors.margins: 10
                                    color: Theme.fg
                                    font.pixelSize: 12
                                    echoMode: TextInput.Password
                                    text: root.universalKeyInput
                                    onTextChanged: root.autoDetectKey(text)
                                }
                            }

                            Txt {
                                text: root.keyDetectStatus
                                color: Theme.success
                                font.pixelSize: 11
                                font.bold: true
                                visible: root.keyDetectStatus.length > 0
                            }
                        }

                        // Provider Buttons (Manual override)
                        Txt {
                            text: "Active Provider (Auto-Selected or Manual):"
                            color: Theme.fg2
                            font.pixelSize: 12
                            font.bold: true
                        }

                        Grid {
                            columns: 3
                            spacing: 6
                            width: parent.width

                            function setProv(p, m, ep) {
                                root.activeProvider = p
                                root.activeModel = m
                                if (ep) root.customEndpoint = ep
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "gemini" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "gemini" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Google Gemini"
                                    color: root.activeProvider === "gemini" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("gemini", "gemini-2.5-flash")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "openai" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "openai" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "OpenAI ChatGPT"
                                    color: root.activeProvider === "openai" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("openai", "gpt-4o-mini")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "claude" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "claude" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Claude Anthropic"
                                    color: root.activeProvider === "claude" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("claude", "claude-3-5-sonnet-latest")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "groq" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "groq" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Groq"
                                    color: root.activeProvider === "groq" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("groq", "llama-3.3-70b-versatile", "https://api.groq.com/openai/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "ollama" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "ollama" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Ollama Local"
                                    color: root.activeProvider === "ollama" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("ollama", "llama3")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "openrouter" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "openrouter" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "OpenRouter"
                                    color: root.activeProvider === "openrouter" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("openrouter", "meta-llama/llama-3.3-70b-instruct:free", "https://openrouter.ai/api/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "deepseek" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "deepseek" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "DeepSeek"
                                    color: root.activeProvider === "deepseek" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("deepseek", "deepseek-chat", "https://api.deepseek.com/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "mistral" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "mistral" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Mistral"
                                    color: root.activeProvider === "mistral" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("mistral", "mistral-small-latest", "https://api.mistral.ai/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "xai" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "xai" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "xAI Grok"
                                    color: root.activeProvider === "xai" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("xai", "grok-2-latest", "https://api.x.ai/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "together" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "together" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Together AI"
                                    color: root.activeProvider === "together" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("together", "meta-llama/Llama-3.3-70B-Instruct-Turbo", "https://api.together.xyz/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "perplexity" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "perplexity" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Perplexity"
                                    color: root.activeProvider === "perplexity" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("perplexity", "sonar", "https://api.perplexity.ai/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "zhipu" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "zhipu" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Zhipu GLM"
                                    color: root.activeProvider === "zhipu" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("zhipu", "glm-4-flash", "https://open.bigmodel.cn/api/paas/v4/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "siliconflow" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "siliconflow" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "SiliconFlow"
                                    color: root.activeProvider === "siliconflow" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("siliconflow", "Qwen/Qwen2.5-7B-Instruct", "https://api.siliconflow.cn/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "moonshot" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "moonshot" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Kimi Moonshot"
                                    color: root.activeProvider === "moonshot" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("moonshot", "moonshot-v1-8k", "https://api.moonshot.cn/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "qwen" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "qwen" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Qwen"
                                    color: root.activeProvider === "qwen" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("qwen", "qwen-plus", "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "fireworks" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "fireworks" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Fireworks"
                                    color: root.activeProvider === "fireworks" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("fireworks", "accounts/fireworks/models/llama-v3p1-8b-instruct", "https://api.fireworks.ai/inference/v1/chat/completions")
                                }
                            }

                            Rectangle {
                                width: 140
                                height: 32
                                radius: 8
                                color: root.activeProvider === "custom" ? Theme.acc : Theme.bg2
                                border.color: root.activeProvider === "custom" ? Theme.acc : Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "Custom API"
                                    color: root.activeProvider === "custom" ? Theme.sfg : Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: parent.parent.setProv("custom", "llama3-8b-8192")
                                }
                            }
                        }

                        // Model Name Input
                        Column {
                            width: parent.width
                            spacing: 4
                            Txt { text: "Model Identifier:"; color: Theme.fg2; font.pixelSize: 11 }
                            Rectangle {
                                width: parent.width
                                height: 36
                                radius: 8
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                TxtInput {
                                    anchors.fill: parent
                                    anchors.margins: 8
                                    color: Theme.fg
                                    font.pixelSize: 12
                                    text: root.activeModel
                                    onTextChanged: root.activeModel = text
                                }
                            }
                        }

                        // Custom Endpoint (OpenAI-compatible providers)
                        Column {
                            width: parent.width
                            spacing: 4
                            visible: root.activeProvider === "custom"
                            Txt { text: "Custom Endpoint (OpenAI-compatible):"; color: Theme.fg2; font.pixelSize: 11 }
                            Rectangle {
                                width: parent.width
                                height: 36
                                radius: 8
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                TxtInput {
                                    anchors.fill: parent
                                    anchors.margins: 8
                                    color: Theme.fg
                                    font.pixelSize: 12
                                    text: root.customEndpoint
                                    onTextChanged: root.customEndpoint = text
                                }
                            }
                        }

                        // Persona Preset Selection
                        Column {
                            width: parent.width
                            spacing: 4
                            Txt { text: "System Persona:"; color: Theme.fg2; font.pixelSize: 11 }

                            Row {
                                spacing: 6
                                function setPersona(p) { root.systemPersona = p }

                                Rectangle {
                                    width: 110
                                    height: 28
                                    radius: 6
                                    color: root.systemPersona === "General Assistant" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "General"
                                        color: root.systemPersona === "General Assistant" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setPersona("General Assistant")
                                    }
                                }
                                Rectangle {
                                    width: 120
                                    height: 28
                                    radius: 6
                                    color: root.systemPersona === "Linux & Hyprland Expert" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "Linux Expert"
                                        color: root.systemPersona === "Linux & Hyprland Expert" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setPersona("Linux & Hyprland Expert")
                                    }
                                }
                                Rectangle {
                                    width: 110
                                    height: 28
                                    radius: 6
                                    color: root.systemPersona === "Senior Code Developer" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "Developer"
                                        color: root.systemPersona === "Senior Code Developer" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setPersona("Senior Code Developer")
                                    }
                                }
                                Rectangle {
                                    width: 100
                                    height: 28
                                    radius: 6
                                    color: root.systemPersona === "Concise Answerer" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "Concise"
                                        color: root.systemPersona === "Concise Answerer" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setPersona("Concise Answerer")
                                    }
                                }
                            }
                        }

                        // === Simulate Browser (web login session) ===
                        Column {
                            width: parent.width
                            spacing: 8

                            Row {
                                width: parent.width
                                spacing: 8

                                Txt {
                                    text: "🖥️ Simulate Browser"
                                    color: Theme.info
                                    font.pixelSize: 12
                                    font.bold: true
                                    anchors.verticalCenter: parent.verticalCenter
                                }

                                ToggleSwitch {
                                    anchors.verticalCenter: parent.verticalCenter
                                    checked: root.browserMode
                                    onToggled: function(v) {
                                        root.browserMode = v
                                        if (v) root.browserProvider = root.activeProvider
                                    }
                                }
                            }

                            Txt {
                                width: parent.width
                                text: root.browserMode
                                    ? "ON — chat as a logged-in browser session instead of an API key. Requests replay your saved cookies."
                                    : "OFF — log in through your real browser, save the session here, and the AI service will think you are the browser."
                                color: Theme.fg3
                                font.pixelSize: 11
                                wrapMode: Text.WordWrap
                            }

                            Txt {
                                text: "Service to simulate:"
                                color: Theme.fg2
                                font.pixelSize: 10
                                font.bold: true
                            }

                            Row {
                                spacing: 6
                                function setSim(p) { root.browserProvider = p }

                                Rectangle {
                                    width: 90
                                    height: 28
                                    radius: 6
                                    color: root.browserProvider === "gemini" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "Gemini Web"
                                        color: root.browserProvider === "gemini" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setSim("gemini")
                                    }
                                }
                                Rectangle {
                                    width: 90
                                    height: 28
                                    radius: 6
                                    color: root.browserProvider === "openai" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "ChatGPT Web"
                                        color: root.browserProvider === "openai" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setSim("openai")
                                    }
                                }
                                Rectangle {
                                    width: 90
                                    height: 28
                                    radius: 6
                                    color: root.browserProvider === "claude" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "Claude Web"
                                        color: root.browserProvider === "claude" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setSim("claude")
                                    }
                                }
                                Rectangle {
                                    width: 90
                                    height: 28
                                    radius: 6
                                    color: root.browserProvider === "custom" ? Theme.acc : Theme.bg2
                                    Txt {
                                        anchors.centerIn: parent
                                        text: "Custom"
                                        color: root.browserProvider === "custom" ? Theme.sfg : Theme.fg
                                        font.pixelSize: 11
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: parent.parent.setSim("custom")
                                    }
                                }
                            }

                            Rectangle {
                                width: parent.width
                                height: 34
                                radius: 8
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                Txt {
                                    anchors.centerIn: parent
                                    text: "🔗 Open Login Page in Browser"
                                    color: Theme.fg
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.openBrowserLogin()
                                }
                            }

                            Txt {
                                width: parent.width
                                text: "After logging in: DevTools (F12) → Network → click any request → right-click → Copy as cURL → paste below (or just paste the Cookie header)."
                                color: Theme.fg3
                                font.pixelSize: 10
                                wrapMode: Text.WordWrap
                            }

                            Rectangle {
                                width: parent.width
                                height: 56
                                radius: 8
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                TextEdit {
                                    anchors.fill: parent
                                    anchors.margins: 8
                                    color: Theme.fg
                                    font.pixelSize: 11
                                    wrapMode: Text.WrapAnywhere
                                    selectByMouse: true
                                    text: root.browserSession
                                    onTextChanged: root.browserSession = text
                                }
                            }

                            Column {
                                width: parent.width
                                spacing: 4
                                Txt { text: "Web Endpoint (optional, for custom/OpenAI-compatible gateways):"; color: Theme.fg2; font.pixelSize: 10 }
                                Rectangle {
                                    width: parent.width
                                    height: 32
                                    radius: 8
                                    color: Theme.bg2
                                    border.color: Theme.border
                                    border.width: 1
                                    TxtInput {
                                        anchors.fill: parent
                                        anchors.margins: 8
                                        color: Theme.fg
                                        font.pixelSize: 11
                                        text: root.browserEndpoint
                                        onTextChanged: root.browserEndpoint = text
                                    }
                                }
                            }

                            Txt {
                                text: root.browserStatus
                                color: Theme.success
                                font.pixelSize: 11
                                font.bold: true
                                wrapMode: Text.WordWrap
                                width: parent.width
                                visible: root.browserStatus.length > 0
                            }

                            Rectangle {
                                width: parent.width
                                height: 34
                                radius: 8
                                color: Theme.acc
                                Txt {
                                    anchors.centerIn: parent
                                    text: "💾 Save Browser Session"
                                    color: Theme.sfg
                                    font.pixelSize: 12
                                    font.bold: true
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        root.browserStatus = "Browser session saved!"
                                        root.saveConfig()
                                    }
                                }
                            }
                        }

                        // Save & Apply Button
                        Rectangle {
                            width: parent.width
                            height: 38
                            radius: 10
                            color: Theme.acc
                            Txt {
                                anchors.centerIn: parent
                                text: "💾 Save Settings"
                                color: Theme.sfg
                                font.family: Theme.fontName
                                font.pixelSize: 13
                                font.bold: true
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.saveConfig()
                                    root.showSettings = false
                                }
                            }
                        }
                    }
                }

                ScrollDragger {
                    target: settingsFlick
                    visible: root.showSettings
                    thumbWidth: 4
                    minThumbHeight: 26
                }

                // === View 2: Chat Interface ===
                Column {
                    anchors.fill: parent
                    visible: !root.showSettings
                    spacing: 8

                    // Quick Prompt Chips Bar (End-4 style)
                    Flickable {
                        width: parent.width
                        height: 32
                        contentWidth: chipRow.width
                        boundsBehavior: Flickable.StopAtBounds
                        clip: true

                        Row {
                            id: chipRow
                            spacing: 6

                            function triggerChip(promptText) {
                                root.sendPrompt(promptText)
                            }

                            Rectangle {
                                height: 28
                                width: 145
                                radius: 14
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                Row {
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Txt { text: "⚡"; font.pixelSize: 12 }
                                    Txt { text: "Summarize clipboard"; color: Theme.fg; font.pixelSize: 11 }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.pasteClipboardToSend()
                                }
                            }

                            Rectangle {
                                height: 28
                                width: 130
                                radius: 14
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                Row {
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Txt { text: "💻"; font.pixelSize: 12 }
                                    Txt { text: "Debug bash script"; color: Theme.fg; font.pixelSize: 11 }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: parent.parent.triggerChip("Help debug and optimize my current bash script or shell function.")
                                }
                            }

                            Rectangle {
                                height: 28
                                width: 135
                                radius: 14
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                Row {
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Txt { text: "📝"; font.pixelSize: 12 }
                                    Txt { text: "Explain command"; color: Theme.fg; font.pixelSize: 11 }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: parent.parent.triggerChip("Explain how this Linux command works step by step:")
                                }
                            }

                            Rectangle {
                                height: 28
                                width: 145
                                radius: 14
                                color: Theme.bg2
                                border.color: Theme.border
                                border.width: 1
                                Row {
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Txt { text: "⚙️"; font.pixelSize: 12 }
                                    Txt { text: "Hyprland Keybinds"; color: Theme.fg; font.pixelSize: 11 }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: parent.parent.triggerChip("Give me recommended Hyprland keybind configuration examples.")
                                }
                            }
                        }
                    }

                    // Empty State Hero
                    Item {
                        width: parent.width
                        height: parent.height - 40
                        visible: root.messages.length === 0

                        Column {
                            anchors.centerIn: parent
                            spacing: 12
                            width: parent.width - 40

                            Rectangle {
                                width: 56
                                height: 56
                                radius: 18
                                color: Theme.acc
                                anchors.horizontalCenter: parent.horizontalCenter
                                Txt {
                                    anchors.centerIn: parent
                                    text: "󰚩"
                                    color: Theme.sfg
                                    font.family: Theme.iconFont
                                    font.pixelSize: 28
                                }
                            }

                            Txt {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: "How can I help you today?"
                                color: Theme.fg
                                font.family: Theme.fontName
                                font.pixelSize: 16
                                font.bold: true
                            }

                            Txt {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: "Ask anything, summarize clipboard, write code or configure your Linux shell."
                                color: Theme.fg3
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                wrapMode: Text.WordWrap
                                horizontalAlignment: Text.AlignHCenter
                                width: parent.width
                            }
                        }
                    }

                    // Message List
                    Item {
                        width: parent.width
                        height: parent.height - 40
                        visible: root.messages.length > 0

                        ListView {
                            id: chatListView
                            anchors.fill: parent
                            anchors.rightMargin: 4
                            clip: true
                            spacing: 10
                            model: root.messages

                            delegate: Item {
                                id: msgDelegate
                                required property int index
                                required property var modelData
                                width: chatListView.width
                                height: msgCol.height + 8

                                Column {
                                    id: msgCol
                                    width: parent.width
                                    spacing: 4

                                    // User Message (Right Aligned)
                                    Row {
                                        width: parent.width
                                        visible: msgDelegate.modelData.sender === "user"
                                        layoutDirection: Qt.RightToLeft

                                        Rectangle {
                                            width: Math.min(parent.width * 0.8, userTxt.implicitWidth + 24)
                                            height: userTxt.implicitHeight + 16
                                            radius: 16
                                            color: Theme.acc

                                            Txt {
                                                id: userTxt
                                                anchors.fill: parent
                                                anchors.margins: 10
                                                text: msgDelegate.modelData.text
                                                color: Theme.sfg
                                                font.family: Theme.fontName
                                                font.pixelSize: 13
                                                wrapMode: Text.WordWrap
                                            }
                                        }
                                    }

                                    // Assistant / System Message (Left Aligned)
                                    Row {
                                        width: parent.width
                                        visible: msgDelegate.modelData.sender !== "user"
                                        spacing: 8

                                        Rectangle {
                                            width: 26
                                            height: 26
                                            radius: 8
                                            color: Theme.bg2
                                            border.color: Theme.border
                                            border.width: 1
                                            anchors.top: parent.top

                                            Txt {
                                                anchors.centerIn: parent
                                                text: msgDelegate.modelData.sender === "assistant" ? "󰚩" : "⚠️"
                                                color: msgDelegate.modelData.sender === "assistant" ? Theme.info : Theme.warning
                                                font.family: Theme.iconFont
                                                font.pixelSize: 13
                                            }
                                        }

                                        Column {
                                            width: parent.width - 36
                                            spacing: 4

                                            Rectangle {
                                                width: parent.width
                                                height: aiTxt.implicitHeight + 20
                                                radius: 14
                                                color: Theme.bg2
                                                border.color: Theme.border
                                                border.width: 1

                                                Txt {
                                                    id: aiTxt
                                                    anchors.fill: parent
                                                    anchors.margins: 10
                                                    text: msgDelegate.modelData.text
                                                    color: Theme.fg
                                                    font.family: Theme.fontName
                                                    font.pixelSize: 13
                                                    wrapMode: Text.WordWrap
                                                    textFormat: Text.MarkdownText
                                                }
                                            }

                                            // Action Row for Assistant Bubble
                                            Row {
                                                spacing: 8
                                                visible: msgDelegate.modelData.sender === "assistant" && msgDelegate.modelData.text !== "..."

                                                IconButton {
                                                    glyph: "󰆏"
                                                    size: 24
                                                    glyphSize: 12
                                                    color: Theme.fg3
                                                    onClicked: root.copyToClipboard(msgDelegate.modelData.text)
                                                }

                                                Txt {
                                                    anchors.verticalCenter: parent.verticalCenter
                                                    text: msgDelegate.modelData.timestamp || ""
                                                    color: Theme.fg3
                                                    font.pixelSize: 10
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        ScrollDragger {
                            target: chatListView
                        }
                    }
                }
            }

            // --- Footer Input Area ---
            Rectangle {
                width: parent.width
                height: 52
                radius: 16
                color: Theme.bg2
                border.color: Theme.border
                border.width: 1

                Row {
                    anchors.fill: parent
                    anchors.margins: 6
                    spacing: 8

                    // Clipboard Paste Button
                    IconButton {
                        glyph: "󰅌"
                        size: 38
                        glyphSize: 14
                        color: Theme.fg2
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: root.pasteClipboardToSend()
                    }

                    // Input Text Field
                    Item {
                        width: parent.width - 100
                        height: parent.height
                        anchors.verticalCenter: parent.verticalCenter

                        Txt {
                            anchors.left: parent.left
                            anchors.leftMargin: 6
                            anchors.verticalCenter: parent.verticalCenter
                            text: "Ask AI anything... (Enter to send)"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            visible: inputEdit.text.length === 0
                        }

                        TxtInput {
                            id: inputEdit
                            anchors.fill: parent
                            anchors.margins: 6
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 13

                            Keys.onReturnPressed: function(event) {
                                if (event.modifiers & Qt.ShiftModifier) {
                                    inputEdit.insert(inputEdit.cursorPosition, "\n")
                                } else {
                                    root.sendPrompt(inputEdit.text)
                                }
                                event.accepted = true
                            }
                        }
                    }

                    // Send or Stop Button
                    Rectangle {
                        width: 38
                        height: 38
                        radius: 12
                        color: root.isGenerating ? Theme.danger : Theme.acc
                        anchors.verticalCenter: parent.verticalCenter

                        Txt {
                            anchors.centerIn: parent
                            text: root.isGenerating ? "󰅙" : "󰒔"
                            color: Theme.sfg
                            font.family: Theme.iconFont
                            font.pixelSize: 16
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.isGenerating) {
                                    root.stopGeneration()
                                } else {
                                    root.sendPrompt(inputEdit.text)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
