"""QuantLib 1.44 reference fixtures for market calendars and schedules."""
import QuantLib as ql


def cases():
    markets = {
        "UnitedStatesSettlement": ql.UnitedStates(ql.UnitedStates.Settlement),
        "Nyse": ql.UnitedStates(ql.UnitedStates.NYSE),
        "GovernmentBond": ql.UnitedStates(ql.UnitedStates.GovernmentBond),
        "FederalReserve": ql.UnitedStates(ql.UnitedStates.FederalReserve),
        "Target": ql.TARGET(),
        "UnitedKingdomExchange": ql.UnitedKingdom(ql.UnitedKingdom.Exchange),
        "Japan": ql.Japan(),
        "Canada": ql.Canada(),
    }
    sample_dates = {
        "UnitedStatesSettlement": [(4, 7, 2024)], "Nyse": [(4, 7, 2024)],
        "GovernmentBond": [(19, 2, 2024)], "FederalReserve": [(19, 2, 2024)],
        "Target": [(25, 12, 2024)], "UnitedKingdomExchange": [(25, 12, 2024)],
        "Japan": [(1, 1, 2024)], "Canada": [(1, 7, 2024)],
    }
    holiday_cases = [{"fn": "holiday", "args": {"calendar": name, "date": ql.Date(*parts).ISO()},
                      "expected": calendar.isHoliday(ql.Date(*parts))}
                     for name, calendar in markets.items() for parts in sample_dates[name]]

    schedules = []
    for effective, termination, frequency, rule, eom in [
        (ql.Date(15, 1, 2024), ql.Date(15, 1, 2025), ql.Period(ql.Quarterly), ql.DateGeneration.Backward, False),
        (ql.Date(31, 1, 2024), ql.Date(31, 1, 2025), ql.Period(ql.Monthly), ql.DateGeneration.Forward, True),
        (ql.Date(15, 1, 2024), ql.Date(30, 11, 2024), ql.Period(ql.Quarterly), ql.DateGeneration.Backward, False),
    ]:
        schedule = ql.Schedule(effective, termination, frequency, ql.TARGET(), ql.ModifiedFollowing,
                               ql.ModifiedFollowing, rule, eom)
        schedules.append({"fn": "schedule", "args": {"effective": effective.ISO(),
            "termination": termination.ISO(), "frequency": "Monthly" if frequency.units() == ql.Months and frequency.length() == 1 else "Quarterly",
            "rule": "Backward" if rule == ql.DateGeneration.Backward else "Forward", "eom": eom},
            "expected": [d.ISO() for d in schedule]})
    dc_cases = []
    for start, end, ref_start, ref_end, freq in [
        (ql.Date(15, 1, 2024), ql.Date(30, 1, 2024), ql.Date(1, 1, 2024), ql.Date(1, 7, 2024), 2),
        (ql.Date(1, 1, 2023), ql.Date(1, 1, 2024), None, None, None),
    ]:
        if freq:
            value = ql.ActualActual(ql.ActualActual.ISMA).yearFraction(start, end, ref_start, ref_end)
            dc_cases.append({"fn": "icma", "args": {"start": start.ISO(), "end": end.ISO(),
                "ref_start": ref_start.ISO(), "ref_end": ref_end.ISO(), "frequency": freq}, "expected": value})
        else:
            value = ql.ActualActual(ql.ActualActual.AFB).yearFraction(start, end)
            dc_cases.append({"fn": "afb", "args": {"start": start.ISO(), "end": end.ISO()}, "expected": value})
    return {"reference": {"QuantLib": "1.44"}, "tolerance": {"icma": {"atol": 1e-14, "rtol": 0.0}, "afb": {"atol": 1e-14, "rtol": 0.0}}, "cases": holiday_cases + schedules + dc_cases}
