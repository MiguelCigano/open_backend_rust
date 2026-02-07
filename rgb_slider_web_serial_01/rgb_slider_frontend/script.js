const rSlider = document.getElementById("r");
const gSlider = document.getElementById("g");
const bSlider = document.getElementById("b");

const rVal = document.getElementById("rVal");
const gVal = document.getElementById("gVal");
const bVal = document.getElementById("bVal");

const preview = document.getElementById("preview");

function updateUI() {
    const r = rSlider.value;
    const g = gSlider.value;
    const b = bSlider.value;

    rVal.textContent = r;
    gVal.textContent = g;
    bVal.textContent = b;

    const color = `rgb(${r}, ${g}, ${b})`;
    
    preview.style.backgroundColor = color;
    document.body.style.background = color;
}

async function sendRGB() {
    const data = new URLSearchParams();
    data.append("red", rSlider.value);
    data.append("green", gSlider.value);
    data.append("blue", bSlider.value);

    try {
        await fetch("/set_rgb", {
            method: "POST",
            body: data
        });
    } catch (err) {
        console.error("Error sending RGB:", err);
    }
}

    function updateRange(el) {
        const min = Number(el.min || 0);
        const max = Number(el.max || 100);
        const val = Number(el.value);
        const pct = (max > min) ? ((val - min) / (max - min) * 100) : 0;
        el.style.setProperty('--range', pct + '%');
    }

function handleChange(e) {
    const el = e && e.target ? e.target : this;
    updateRange(el);
    updateUI();
    sendRGB();
}

rSlider.addEventListener("input", handleChange);
gSlider.addEventListener("input", handleChange);
bSlider.addEventListener("input", handleChange);

updateUI();
    updateRange(rSlider);
    updateRange(gSlider);
    updateRange(bSlider);
