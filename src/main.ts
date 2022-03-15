const apricot: any = require("@apricot-lend/apricot")

async function main() {
    console.log(apricot);
    console.log((await apricot.consts.get_price_pda()).toString())
    console.log("Hello World");
}

main().then(() => {
    console.log('Success')
})


