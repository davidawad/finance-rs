use finance_rs::cashflow::xirr;
use finance_rs::tvm::{pmt, When};
use finance_rs::{Date, FinanceError};
use std::fmt::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), FinanceError> {
    let mut arguments = std::env::args_os().skip(1);
    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| FinanceError::InvalidInput {
            what: "arguments",
            reason: "usage: showcase <svg-output-path>",
        })?;
    if arguments.next().is_some() {
        return Err(FinanceError::InvalidInput {
            what: "arguments",
            reason: "usage: showcase <svg-output-path>",
        });
    }

    let payment = pmt(0.06 / 12.0, 360.0, 300_000.0, 0.0, When::End)?;
    let dates = [
        Date::new(2024, 1, 1)?,
        Date::new(2024, 7, 1)?,
        Date::new(2025, 1, 1)?,
    ];
    let xirr_value = xirr(&dates, &[-100.0, 42.0, 70.0])?;

    let mut svg = String::new();
    svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"540\" viewBox=\"0 0 960 540\">\n");
    svg.push_str("<rect width=\"960\" height=\"540\" rx=\"28\" fill=\"#101827\"/>\n");
    svg.push_str("<text x=\"64\" y=\"83\" fill=\"#f8fafc\" font-family=\"ui-monospace,monospace\" font-size=\"27\" font-weight=\"700\">finance-rs  |  synthetic examples</text>\n");
    svg.push_str("<text x=\"64\" y=\"132\" fill=\"#94a3b8\" font-family=\"ui-monospace,monospace\" font-size=\"18\">Rust financial math, computed by the library</text>\n");
    svg.push_str(
        "<rect x=\"64\" y=\"176\" width=\"832\" height=\"132\" rx=\"16\" fill=\"#1e293b\"/>\n",
    );
    svg.push_str("<text x=\"92\" y=\"220\" fill=\"#38bdf8\" font-family=\"ui-monospace,monospace\" font-size=\"17\">30-YEAR LOAN  |  $300,000  |  6% NOMINAL  |  MONTHLY</text>\n");
    writeln!(svg, "<text x=\"92\" y=\"273\" fill=\"#f8fafc\" font-family=\"ui-monospace,monospace\" font-size=\"31\">payment = ${:.2} / month</text>", payment.abs()).map_err(|_| FinanceError::InvalidInput { what: "SVG", reason: "formatting failed" })?;
    svg.push_str(
        "<rect x=\"64\" y=\"332\" width=\"832\" height=\"132\" rx=\"16\" fill=\"#1e293b\"/>\n",
    );
    svg.push_str("<text x=\"92\" y=\"376\" fill=\"#34d399\" font-family=\"ui-monospace,monospace\" font-size=\"17\">DATED CASH FLOWS  |  $100 INVESTED  |  SYNTHETIC</text>\n");
    writeln!(svg, "<text x=\"92\" y=\"429\" fill=\"#f8fafc\" font-family=\"ui-monospace,monospace\" font-size=\"31\">XIRR = {:.2}% annualized</text>", xirr_value * 100.0).map_err(|_| FinanceError::InvalidInput { what: "SVG", reason: "formatting failed" })?;
    svg.push_str("<text x=\"64\" y=\"505\" fill=\"#64748b\" font-family=\"ui-monospace,monospace\" font-size=\"14\">No client, account, or market data. Values are example inputs.</text>\n</svg>\n");
    std::fs::write(output, svg).map_err(|_| FinanceError::InvalidInput {
        what: "SVG output",
        reason: "write failed",
    })?;
    Ok(())
}
